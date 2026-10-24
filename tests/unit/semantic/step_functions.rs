use crate::{
    model::{Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/step-functions-plan.json");

#[test]
fn step_functions_are_regional_cards_with_safe_role_references() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 3);
    assert_eq!(graph.edges.len(), 1);
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_sfn_"))
    {
        assert_eq!(node.role, ResourceRole::Node);
        assert_eq!(node.placement, Placement::Spatial);
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .unwrap()
                .0,
            "us-east-1"
        );
    }
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.complete && r.label == "Execution role reference")
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("planorama-icon-aws-step-functions"));
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
    }
}

#[test]
fn step_functions_role_proof_rejects_uncertain_and_historical_endpoints() {
    for case in [
        "unknown",
        "mismatch",
        "ambiguous",
        "missing",
        "wrong-provider",
        "wrong-type",
        "delete",
        "deposed",
        "data",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                value["resource_changes"][1]["change"]["after_unknown"] = json!({"role_arn":true})
            }
            "mismatch" => {
                value["resource_changes"][1]["change"]["after"]["role_arn"] = json!("different")
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["role_arn"] = json!({"references":["aws_iam_role.workflow.arn", "aws_sfn_activity.worker.arn"]})
            }
            "missing" => value["resource_changes"][0]["address"] = json!("aws_iam_role.other"),
            "wrong-provider" => {
                value["resource_changes"][0]["provider_name"] =
                    json!("registry.terraform.io/example/aws")
            }
            "wrong-type" => value["resource_changes"][0]["type"] = json!("aws_iam_user"),
            "delete" => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "deposed" => value["resource_changes"][0]["deposed"] = json!("old"),
            "data" => value["resource_changes"][0]["mode"] = json!("data"),
            _ => unreachable!(),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .all(|r| !r.service_reference_verified),
            "{case}"
        );
    }
}

#[test]
fn step_functions_keep_indexed_and_filtered_state_identity() {
    let indexed = FIXTURE.replace("aws_iam_role.workflow", "aws_iam_role.workflow[0]");
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.complete && r.label == "Execution role reference")
    );
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_sfn_state_machine.workflow".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
