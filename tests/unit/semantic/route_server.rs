use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/route-server-plan.json");

#[test]
fn dedicated_outputs_prove_membership_and_associations_without_generated_children() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, output, label) in [
        (2, 4, "route_server_id", "route_server_id", "Route server"),
        (
            4,
            5,
            "route_server_endpoint_id",
            "route_server_endpoint_id",
            "BGP peer endpoint",
        ),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_current_scalar_fixture(
            &pair.to_string(),
            label,
            path,
            output,
        );
    }
}

#[test]
fn endpoints_require_subnet_and_unique_coherent_vpc_association() {
    for case in [
        "valid",
        "subnet-vpc",
        "endpoint-vpc",
        "missing-association",
        "unknown",
        "wrong-output",
        "literal",
        "data",
        "deleted",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "subnet-vpc" => v["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("other"),
            "endpoint-vpc" => {
                v["resource_changes"][4]["change"]["after"]["vpc_id"] = json!("other")
            }
            "missing-association" => {
                v["configuration"]["root_module"]["resources"][3]["expressions"] = json!({})
            }
            "unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"subnet_id":true})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["route_server_id"] =
                    json!({"references":["aws_vpc_route_server.main.id"]})
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"] = json!({})
            }
            "data" => v["resource_changes"][1]["mode"] = json!("data"),
            "deleted" => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 8);
        assert_eq!(
            g.edges.iter().any(
                |e| g.nodes[e.to].resource_type == "aws_vpc_route_server_endpoint"
                    && e.kind == crate::model::EdgeKind::Containment
            ),
            case == "valid",
            "{case}"
        );
        assert!(!g.nodes.iter().any(|n| matches!(
            n.resource_type.as_str(),
            "aws_network_interface" | "aws_sns_topic"
        )));
    }
}

#[test]
fn propagation_vpc_and_late_account_evidence_remain_independent() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][6]["change"]["after"]["vpc_id"] = json!("other");
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        g.nodes
            .iter()
            .any(|n| n.resource_type == "aws_vpc_route_server_propagation")
    );
    let mut p = plan::parse(FIXTURE).unwrap();
    for node in &mut p.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_vpc_route_server_vpc_association")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(!g.edges.iter().any(|e| g.nodes[e.to].resource_type
        == "aws_vpc_route_server_endpoint"
        && e.kind == crate::model::EdgeKind::Containment));
}
