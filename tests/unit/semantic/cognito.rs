use crate::{model::ResourceRole, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/cognito-plan.json");

#[test]
fn cognito_selection_indexed_identity_and_state_boundaries() {
    let indexed = FIXTURE.replace(
        "aws_cognito_user_pool.users",
        "aws_cognito_user_pool.users[0]",
    );
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    assert_eq!(graph.coverage.selected.resources, 3);
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_cognito_user_pool_client.app")
            .unwrap()
            .references
            .iter()
            .any(|r| r.complete && r.label == "User pool reference")
    );
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_cognito_user_pool_client.app".into()),
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
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let second =
        value["resource_changes"][2]["change"]["after"]["cognito_identity_providers"][0].clone();
    value["resource_changes"][2]["change"]["after"]["cognito_identity_providers"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(
        parsed
            .attributes
            .iter()
            .filter(|r| r.attribute.starts_with("cognito_identity_providers."))
            .all(|r| !r.service_reference_verified)
    );
}

#[test]
fn cognito_preserves_identity_and_verified_references_without_secrets() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 3);
    assert!(graph.nodes.iter().all(|n| n.role == ResourceRole::Node));
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.complete && r.label != "References")
            .count(),
        3
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
        assert!(output.contains("planorama-icon-aws-cognito"));
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("fixture-client"));
    }
}

#[test]
fn cognito_rejects_unproven_client_pool_relationships() {
    for case in [
        "unknown",
        "mismatch",
        "ambiguous",
        "missing",
        "wrong-provider",
        "delete",
        "deposed",
        "data",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                value["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"user_pool_id":true})
            }
            "mismatch" => {
                value["resource_changes"][1]["change"]["after"]["user_pool_id"] = json!("different")
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["user_pool_id"] = json!({"references":["aws_cognito_user_pool.users.id", "aws_cognito_identity_pool.identities.id"]})
            }
            "missing" => {
                value["resource_changes"].as_array_mut().unwrap()[0]["address"] =
                    json!("aws_cognito_user_pool.other")
            }
            "wrong-provider" => {
                value["resource_changes"][0]["provider_name"] =
                    json!("registry.terraform.io/example/aws")
            }
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
                .filter(|r| r.attribute == "user_pool_id")
                .all(|r| !r.service_reference_verified),
            "{case}"
        );
    }
}
