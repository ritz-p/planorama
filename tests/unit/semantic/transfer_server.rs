use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/transfer-server-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn complete(v: &Value, t: usize, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == label && r.complete)
}
#[test]
fn all_server_modes_have_independent_cards_and_only_verified_network_context() {
    let v = fixture();
    for l in [
        "Transfer endpoint VPC",
        "Transfer endpoint subnets",
        "Transfer endpoint security groups",
        "Transfer endpoint Elastic IPs",
        "Transfer FTPS certificate",
        "Transfer identity Lambda",
        "Transfer logging role",
        "Transfer structured log groups",
        "Upload workflow",
        "Upload workflow role",
        "Partial upload workflow",
        "Partial upload workflow role",
    ] {
        assert!(complete(&v, 12, l), "{l}");
    }
    for (t, l) in [
        (13, "Transfer VPC endpoint"),
        (13, "Transfer identity directory"),
        (14, "Transfer identity invocation role"),
    ] {
        assert!(complete(&v, t, l), "{l}");
    }
    assert!(!complete(&v, 12, "Transfer VPC endpoint"));
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 15);
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for (i, n) in g
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_transfer_server")
    {
        assert_eq!(n.placement, crate::model::Placement::Spatial);
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.to == i && e.kind == crate::model::EdgeKind::Containment),
            n.address == "aws_transfer_server.vpc"
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
        assert!(!out.contains("PRIVATE_"));
    }
}
#[test]
fn explicit_server_roles_and_certificates_preserve_lifecycle_selection_and_states() {
    for (s, path, l) in [
        (4, "certificate", "Transfer FTPS certificate"),
        (5, "logging_role", "Transfer logging role"),
        (6, "function", "Transfer identity Lambda"),
    ] {
        let v = fixture();
        let mut pair = v.clone();
        pair["resource_changes"] = json!([v["resource_changes"][s], v["resource_changes"][12]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][s],
            v["configuration"]["root_module"]["resources"][12]
        ]);
        super::super::service_references::tests::check_service_fixture(&pair.to_string(), l, path);
    }
}
#[test]
fn endpoint_modes_and_identity_provider_modes_are_not_interchangeable() {
    for mode in ["PUBLIC", "VPC_ENDPOINT", "VPC", "unknown"] {
        let mut v = fixture();
        v["resource_changes"][12]["change"]["after"]["endpoint_type"] = json!(mode);
        v["resource_changes"][13]["change"]["after"]["endpoint_type"] = json!(mode);
        assert_eq!(complete(&v, 12, "Transfer endpoint subnets"), mode == "VPC");
        assert_eq!(
            complete(&v, 13, "Transfer VPC endpoint"),
            mode == "VPC_ENDPOINT"
        );
    }
    for mode in [
        "AWS_DIRECTORY_SERVICE",
        "AWS_LAMBDA",
        "API_GATEWAY",
        "SERVICE_MANAGED",
    ] {
        let mut v = fixture();
        for t in 12..15 {
            v["resource_changes"][t]["change"]["after"]["identity_provider_type"] = json!(mode);
        }
        assert_eq!(
            complete(&v, 12, "Transfer identity Lambda"),
            mode == "AWS_LAMBDA"
        );
        assert_eq!(
            complete(&v, 13, "Transfer identity directory"),
            mode == "AWS_DIRECTORY_SERVICE"
        );
        assert_eq!(
            complete(&v, 14, "Transfer identity invocation role"),
            mode == "API_GATEWAY"
        );
    }
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after"]["vpc_endpoint_type"] = json!("Gateway");
    assert!(!complete(&v, 13, "Transfer VPC endpoint"));
}
#[test]
fn network_collections_require_complete_common_vpc_and_known_ipv4_address_mode() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("other"),
            2 => {
                v["resource_changes"][12]["change"]["after"]["endpoint_details"][0]["subnet_ids"] =
                    json!(["subnet-transfer", "literal"])
            }
            3 => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"endpoint_details":[{"subnet_ids":true}]})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["endpoint_details"]
                    [0]["vpc_id"] = json!({"constant_value":"vpc-transfer"})
            }
            5 => {
                v["resource_changes"][12]["change"]["after"]["endpoint_details"] =
                    json!([{ "vpc_id":"vpc-transfer"},{"vpc_id":"vpc-transfer"}])
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 12, "Transfer endpoint subnets"),
            case == 0,
            "{case}"
        );
        assert_eq!(
            complete(&v, 12, "Transfer endpoint Elastic IPs"),
            case == 0,
            "{case}"
        );
    }
    for mode in [json!("DUALSTACK"), Value::Null] {
        let mut v = fixture();
        v["resource_changes"][12]["change"]["after"]["ip_address_type"] = mode;
        assert!(!complete(&v, 12, "Transfer endpoint Elastic IPs"));
    }
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other");
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let t = g
        .nodes
        .iter()
        .position(|n| n.address == "aws_transfer_server.vpc")
        .unwrap();
    assert!(
        !g.edges
            .iter()
            .any(|e| e.to == t && e.kind == crate::model::EdgeKind::Containment)
    );
}
#[test]
fn ftps_certificate_is_acm_only_and_unknown_modes_never_supply_proof() {
    for case in 0..6 {
        let mut v = fixture();
        match case {
            1 => v["resource_changes"][4]["type"] = json!("aws_transfer_certificate"),
            2 => v["resource_changes"][12]["change"]["after"]["protocols"] = json!(["SFTP"]),
            3 => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"protocols":[false,true]})
            }
            4 => v["resource_changes"][12]["change"]["after_sensitive"] = json!({"protocols":true}),
            5 => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["certificate"] =
                    json!({"references":["aws_acm_certificate.transfer.id"]})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 12, "Transfer FTPS certificate"),
            case == 0,
            "{case}"
        );
    }
}
#[test]
fn workflow_blocks_are_singleton_correlated_and_workflow_id_is_not_arn() {
    for case in 0..5 {
        let mut v = fixture();
        match case {
            1 => {
                v["resource_changes"][12]["change"]["after"]["workflow_details"][0]["on_upload"] = json!([{ "workflow_id":"w-upload","execution_role":"role"},{"workflow_id":"w-upload","execution_role":"role"}])
            }
            2 => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["workflow_details"]
                    [0]["on_upload"][0]["workflow_id"] =
                    json!({"references":["aws_transfer_workflow.partial.id"]})
            }
            3 => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"workflow_details":[{"on_upload":[{"workflow_id":true}]}]})
            }
            4 => v["resource_changes"][9]["deposed"] = json!("old"),
            _ => {}
        }
        assert_eq!(complete(&v, 12, "Upload workflow"), case == 0, "{case}");
        assert!(complete(&v, 12, "Partial upload workflow"));
    }
}
#[test]
fn log_destination_suffix_is_exact_and_collections_cannot_hide_unknown_or_literal_values() {
    for case in 0..6 {
        let mut v = fixture();
        let arn = v["resource_changes"][8]["change"]["after"]["arn"]
            .as_str()
            .unwrap()
            .to_string();
        match case {
            1 => {
                v["resource_changes"][12]["change"]["after"]["structured_log_destinations"] =
                    json!([format!("{arn}:stream")])
            }
            2 => {
                v["resource_changes"][12]["change"]["after"]["structured_log_destinations"] =
                    json!([format!("{arn}:*"), "literal"])
            }
            3 => {
                v["resource_changes"][12]["change"]["after_sensitive"] =
                    json!({"structured_log_destinations":[true]})
            }
            4 => v["resource_changes"][8]["type"] = json!("aws_s3_bucket"),
            5 => {
                v["resource_changes"][12]["change"]["after"]["structured_log_destinations"] =
                    json!([arn])
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, 12, "Transfer structured log groups"),
            matches!(case, 0 | 5),
            "{case}"
        );
    }
}
#[test]
fn regional_references_and_global_roles_recheck_compatible_accounts() {
    for case in 0..5 {
        let mut p = plan::parse(FIXTURE).unwrap();
        for n in &mut p.nodes {
            let c = n.provider_configuration.as_mut().unwrap();
            c.account = crate::model::AwsAccountScope::new("111111111111");
            if n.resource_type == "aws_transfer_workflow" {
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
        let n = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_transfer_server.vpc")
            .unwrap();
        assert_eq!(
            n.references
                .iter()
                .any(|r| r.label == "Upload workflow" && r.complete),
            case == 0,
            "{case}"
        );
        assert!(
            n.references
                .iter()
                .any(|r| r.label == "Upload workflow role" && r.complete)
        );
    }
}
