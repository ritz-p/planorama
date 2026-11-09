use crate::plan::evidence::tests::parse_fixture as parse;
use crate::semantic;
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/direct-connect-hosted-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    let g = semantic::transform(&parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn hosted_allocation_acceptance_and_both_gateway_variants_are_explicit() {
    for (t, label) in [
        (3, "Hosted connection LAG"),
        (4, "Confirmed hosted connection"),
        (5, "Allocated interface connection"),
        (6, "Accepted hosted interface"),
        (6, "Direct Connect gateway"),
        (7, "Allocated interface connection"),
        (8, "Accepted hosted interface"),
        (9, "Allocated interface connection"),
        (10, "Accepted hosted interface"),
        (10, "Direct Connect gateway"),
        (11, "Direct Connect gateway"),
        (11, "Associated regional gateway"),
        (13, "Direct Connect gateway"),
        (13, "Associated regional gateway"),
        (14, "Direct Connect gateway"),
        (14, "Accepted gateway proposal"),
    ] {
        assert!(complete(&fixture(), t, label), "{t} {label}");
    }
}
#[test]
fn hosted_account_proof_is_independent_and_rechecked_after_annotation() {
    for case in 0..5 {
        let mut p = parse(FIXTURE).unwrap();
        let n = p
            .nodes
            .iter_mut()
            .find(|n| n.address == "aws_dx_hosted_private_virtual_interface_accepter.customer")
            .unwrap();
        let c = n.provider_configuration.as_mut().unwrap();
        match case {
            1 => c.account = crate::model::AwsAccountScope::new("333333333333"),
            2 => c.account = None,
            3 => c.account_conflict = true,
            4 => c.region = Some(crate::model::RegionScope("us-west-2".into())),
            _ => {}
        }
        let g = semantic::transform(&p);
        let n = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_dx_hosted_private_virtual_interface_accepter.customer")
            .unwrap();
        assert_eq!(
            n.references
                .iter()
                .any(|r| r.label == "Accepted hosted interface" && r.complete),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn hosted_region_never_uses_deprecated_region_or_allocator_provider_region() {
    for case in 0..5 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][3]["change"]["after"]["connection_region"] = Value::Null,
            2 => {
                v["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"connection_region":true})
            }
            3 => {
                v["resource_changes"][3]["change"]["after"]["connection_region"] =
                    json!("us-west-2")
            }
            4 => {
                v["resource_changes"][3]["change"]["after_sensitive"] =
                    json!({"connection_region":true})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 3, "Hosted connection LAG"),
            case == 0,
            "{case}"
        );
        assert_eq!(
            complete(&v, 4, "Confirmed hosted connection"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn hosted_owner_masks_endpoint_variants_and_lifecycle_remain_conservative() {
    for case in 0..8 {
        let mut v = fixture();
        match case {
            0 => {}
            1 => {
                v["resource_changes"][5]["change"]["after"]["owner_account_id"] =
                    json!("333333333333")
            }
            2 => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"owner_account_id":true})
            }
            3 => {
                v["resource_changes"][5]["change"]["after_sensitive"] =
                    json!({"owner_account_id":true})
            }
            4 => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][5]["deposed"] = json!("old"),
            6 => v["resource_changes"][5]["type"] = json!("aws_dx_hosted_public_virtual_interface"),
            7 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["virtual_interface_id"] =
                    json!({"references":["aws_dx_hosted_private_virtual_interface.customer.arn"]})
            }
            _ => unreachable!(),
        }
        assert_eq!(
            complete(&v, 6, "Accepted hosted interface"),
            case == 0,
            "{case}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["vpn_gateway_id"] = json!("vgw-customer");
    assert!(!complete(&v, 6, "Direct Connect gateway"));
    let mut v = fixture();
    v["resource_changes"][0]["type"] = json!("aws_dx_connection");
    assert!(!complete(&v, 3, "Hosted connection LAG"));
}
#[test]
fn cross_account_proposals_require_both_owner_accounts_and_exact_gateway_context() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][14]["change"]["after"]["associated_gateway_owner_account_id"] =
                    json!("333333333333")
            }
            2 => {
                v["resource_changes"][13]["change"]["after_unknown"] =
                    json!({"dx_gateway_owner_account_id":true})
            }
            3 => v["resource_changes"][13]["change"]["after"]["dx_gateway_id"] = json!("wrong"),
            4 => {
                v["configuration"]["root_module"]["resources"][14]["expressions"]["associated_gateway_id"] =
                    json!({"references":["aws_ec2_transit_gateway.partner.id"]})
            }
            5 => v["resource_changes"][13]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        assert_eq!(
            complete(&v, 14, "Accepted gateway proposal"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn acceptance_preserves_shared_identities_and_private_payloads() {
    let g = semantic::transform(&parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 15);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
    }
    let hosted = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_dx_hosted_connection")
        .unwrap();
    assert!(
        hosted
            .entity
            .scope
            .as_ref()
            .and_then(|s| s.region.as_ref())
            .is_none()
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
        assert!(!out.contains("PRIVATE_HOSTED_DX_PAYLOAD"));
        assert!(!out.contains("computed-only"));
    }
}
