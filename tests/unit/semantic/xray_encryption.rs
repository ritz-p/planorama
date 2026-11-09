use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/xray-encryption-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == "aws_xray_encryption_config.customer")
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == "X-Ray customer KMS key" && r.complete)
}
#[test]
fn xray_key_preserves_explicit_identity_lifecycle_selection_and_states() {
    let mut v = fixture();
    v["resource_changes"].as_array_mut().unwrap().truncate(2);
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    super::super::service_references::tests::check_service_fixture(
        &v.to_string(),
        "X-Ray customer KMS key",
        "key_id",
    );
}
#[test]
fn xray_only_known_kms_mode_and_key_arn_are_accepted() {
    for case in 0..9 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][1]["change"]["after"]["type"] = json!("NONE"),
            2 => v["resource_changes"][1]["change"]["after_unknown"] = json!({"type":true}),
            3 => v["resource_changes"][1]["change"]["after_sensitive"] = json!({"type":true}),
            4 => v["resource_changes"][0]["type"] = json!("aws_kms_alias"),
            5 => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["key_id"] =
                    json!({"references":["aws_kms_key.xray.id"]})
            }
            6 => v["resource_changes"][1]["change"]["after"]["key_id"] = json!("xray"),
            7 => v["resource_changes"][1]["change"]["after"]["type"] = json!("kms"),
            8 => v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":true}),
            _ => {}
        }
        assert_eq!(complete(&v), case == 0, "{case}");
    }
}
#[test]
fn xray_account_and_region_are_rechecked_without_scope_inheritance() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_kms_key" {
                match case {
                    1 => c.account = crate::model::AwsAccountScope::new("222222222222"),
                    2 => c.account_conflict = true,
                    3 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
                    4 => c.region = None,
                    _ => {}
                }
            }
        }
        let g = semantic::transform(&p);
        assert_eq!(
            g.nodes
                .iter()
                .find(|n| n.address == "aws_xray_encryption_config.customer")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "X-Ray customer KMS key" && r.complete),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn default_encryption_has_no_customer_key_or_unencrypted_claim() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 3);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    let n = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_xray_encryption_config.default")
        .unwrap();
    assert!(
        !n.references
            .iter()
            .any(|r| r.label == "X-Ray customer KMS key" && r.complete)
    );
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
        assert!(!out.to_lowercase().contains("unencrypted"));
    }
}
