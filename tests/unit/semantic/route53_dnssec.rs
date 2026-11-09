use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/route53-dnssec-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn dnssec_references_preserve_exact_identity_lifecycle_selection_and_state() {
    for (s, t, path, label) in [
        (0, 2, "hosted_zone_id", "DNSSEC hosted zone"),
        (1, 2, "key_management_service_arn", "DNSSEC KMS key"),
        (0, 3, "hosted_zone_id", "DNSSEC hosted zone"),
        (4, 5, "domain_name", "Registered domain owner"),
    ] {
        let v = fixture();
        let mut pair = v.clone();
        pair["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][t]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][s],
            v["configuration"]["root_module"]["resources"][t]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
    }
}
#[test]
fn dnssec_does_not_forward_zone_identity_or_decode_composite_ksk_id() {
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][3]["expressions"]["hosted_zone_id"] =
        json!({"references":["aws_route53_key_signing_key.main.hosted_zone_id"]});
    assert!(!complete(&v, 3, "DNSSEC hosted zone"));
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["id"] = json!("ZMAIN,unrelated-name");
    assert!(complete(&v, 2, "DNSSEC KMS key"));
    v["configuration"]["root_module"]["resources"][2]["expressions"]["key_management_service_arn"] =
        json!({"references":["aws_kms_key.main.id"]});
    assert!(!complete(&v, 2, "DNSSEC KMS key"));
}
#[test]
fn dnssec_kms_requires_east_one_independent_region_and_account() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_kms_key" {
                match case {
                    1 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                    2 => c.region = None,
                    3 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    4 => c.account_conflict = true,
                    _ => {}
                }
            } else {
                c.region = Some(crate::model::RegionScope("eu-west-1".into()));
            }
        }
        let g = semantic::transform(&p);
        let ksk = g
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_route53_key_signing_key")
            .unwrap();
        assert!(ksk.entity.scope.as_ref().unwrap().global);
        assert_eq!(
            ksk.references
                .iter()
                .any(|r| r.label == "DNSSEC KMS key" && r.complete),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn delegation_owner_requires_domain_name_output_from_registration_resource() {
    for kind in [
        "aws_route53domains_registered_domain",
        "aws_route53domains_domain",
        "aws_route53_zone",
        "aws_route53_key_signing_key",
    ] {
        let mut v = fixture();
        v["resource_changes"][4]["type"] = json!(kind);
        assert_eq!(
            complete(&v, 5, "Registered domain owner"),
            kind.starts_with("aws_route53domains_"),
            "{kind}"
        );
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][5]["expressions"]["domain_name"] =
        json!({"constant_value":"example.test"});
    assert!(!complete(&v, 5, "Registered domain owner"));
}
#[test]
fn dnssec_owners_preserve_shared_keys_and_hide_key_material() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 6);
    for kind in [
        "aws_route53_key_signing_key",
        "aws_route53_hosted_zone_dnssec",
        "aws_route53domains_delegation_signer_record",
    ] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_some(),
            "{kind}"
        );
    }
    for kind in [
        "aws_route53_zone",
        "aws_kms_key",
        "aws_route53domains_registered_domain",
    ] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap()
                .member_owner
                .is_none()
        );
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_DNSSEC_PAYLOAD"));
        assert!(!out.contains("computed-key"));
    }
}

#[test]
fn registered_domain_navigation_preserves_independently_verified_account_scope() {
    for kind in [
        "aws_route53domains_registered_domain",
        "aws_route53domains_domain",
    ] {
        let mut v = fixture();
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/fixture"}}]);
        let text = v
            .to_string()
            .replace("aws_route53domains_registered_domain", kind);
        for mismatch in [false, true] {
            let mut p = crate::plan::evidence::tests::parse_fixture(&text).unwrap();
            if mismatch {
                p.nodes
                    .iter_mut()
                    .find(|n| n.resource_type == kind)
                    .unwrap()
                    .provider_configuration
                    .as_mut()
                    .unwrap()
                    .account = crate::model::AwsAccountScope::new("222222222222");
            }
            let g = semantic::transform(&p);
            let owner = g.nodes.iter().find(|n| n.resource_type == kind).unwrap();
            assert!(
                owner.entity.scope.as_ref().unwrap().account.is_some(),
                "{kind}"
            );
            let signer = g
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_route53domains_delegation_signer_record")
                .unwrap();
            assert_eq!(
                signer.member_owner.is_some(),
                !mismatch,
                "{kind} {mismatch}"
            );
            assert!(owner.member_owner.is_none());
        }
    }
}
