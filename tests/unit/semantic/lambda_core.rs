use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/lambda-core-plan.json");

#[test]
fn connector_requires_complete_microvm_subnet_and_security_group_common_vpc_proof() {
    for case in [
        "valid",
        "incomplete",
        "sensitive",
        "unknown",
        "wrong-vpc",
        "literal",
        "compute",
        "shape",
        "deleted",
        "data",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "incomplete" => {
                v["resource_changes"][4]["change"]["after"]["configuration"][0]["vpc_egress_configuration"]
                    [0]["subnet_ids"] = json!(["subnet-a", "literal"])
            }
            "sensitive" => {
                v["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"configuration":true})
            }
            "unknown" => {
                v["resource_changes"][4]["change"]["after_unknown"] = json!({"configuration":true})
            }
            "wrong-vpc" => v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other"),
            "literal" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"] = json!({})
            }
            "compute" => {
                v["resource_changes"][4]["change"]["after"]["configuration"][0]["vpc_egress_configuration"]
                    [0]["associated_compute_resource_types"] = json!(["Other"])
            }
            "shape" => v["resource_changes"][4]["change"]["after"]["configuration"] = json!({}),
            "deleted" => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            "data" => v["resource_changes"][1]["mode"] = json!("data"),
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 6);
        assert_eq!(
            g.edges.iter().any(|e| g.nodes[e.to].resource_type
                == "aws_lambdacore_network_connector"
                && e.kind == crate::model::EdgeKind::Containment),
            case == "valid",
            "{case}"
        );
        assert!(
            !g.nodes
                .iter()
                .any(|n| n.resource_type == "aws_network_interface")
        );
    }
}

#[test]
fn operator_role_requires_current_matching_arn_and_remains_independent() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut pair = v.clone();
    pair["resource_changes"] = json!([v["resource_changes"][5], v["resource_changes"][4]]);
    pair["configuration"]["root_module"]["resources"] = json!([
        v["configuration"]["root_module"]["resources"][5],
        v["configuration"]["root_module"]["resources"][4]
    ]);
    super::super::service_references::tests::check_service_fixture(
        &pair.to_string(),
        "Connector operator role",
        "operator_role",
    );
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(output.content.contains("Egress subnets"));
    }
}

#[test]
fn late_account_evidence_and_named_states_do_not_cross_bind() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_security_group")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(!g.edges.iter().any(|e| g.nodes[e.to].resource_type
        == "aws_lambdacore_network_connector"
        && e.kind == crate::model::EdgeKind::Containment));
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
