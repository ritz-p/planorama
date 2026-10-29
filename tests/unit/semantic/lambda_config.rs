use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/lambda-config-plan.json")).unwrap()
}

fn complete(input: &Value, address: &str, attribute: &str) -> bool {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == attribute
                && !r.dependency
                && r.complete
                && super::label(
                    input["resource_changes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|c| c["address"] == address)
                        .unwrap()["type"]
                        .as_str()
                        .unwrap(),
                    attribute,
                ) == Some(r.label)
        })
}

#[test]
fn lambda_config_typed_references_preserve_shared_independent_resources() {
    let input = fixture();
    for r in input["configuration"]["root_module"]["resources"][0]["expressions"]
        .as_object()
        .unwrap()
        .keys()
    {
        if !r.contains("config") {
            assert!(complete(&input, "aws_lambda_function.app", r), "{r}");
        }
    }
    for field in [
        "dead_letter_config.target_arn",
        "file_system_config.arn",
        "logging_config.log_group",
        "capacity_provider_config.lambda_managed_instances_capacity_provider_config.capacity_provider_arn",
        "vpc_config.subnet_ids",
        "vpc_config.security_group_ids",
    ] {
        assert!(
            complete(&input, "aws_lambda_function.app", field),
            "{field}"
        );
    }
    assert!(complete(
        &input,
        "aws_lambda_code_signing_config.app",
        "allowed_publishers.signing_profile_version_arns"
    ));
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(
        graph.coverage.selected.resources,
        input["resource_changes"].as_array().unwrap().len()
    );
    assert!(
        graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Containment
                && graph.nodes[e.to].resource_type == "aws_lambda_function")
            .all(|e| graph.nodes[e.from].resource_type == "aws_vpc")
    );
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Connection)
            .count(),
        1
    );
    assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_"));
}

#[test]
fn lambda_config_requires_exact_fields_values_lifecycle_and_bindings() {
    for (source, field) in [
        (1, "role"),
        (2, "code_signing_config_arn"),
        (3, "dead_letter_config.target_arn"),
        (4, "file_system_config.arn"),
        (5, "logging_config.log_group"),
        (6, "kms_key_arn"),
        (6, "source_kms_key_arn"),
        (
            7,
            "capacity_provider_config.lambda_managed_instances_capacity_provider_config.capacity_provider_arn",
        ),
    ] {
        for case in 0..10 {
            let mut input = fixture();
            match case {
                0 => input["resource_changes"][source]["change"]["after"] = json!({}),
                1 => input["resource_changes"][source]["mode"] = json!("data"),
                2 => input["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                3 => input["resource_changes"][source]["deposed"] = json!("old"),
                4 => {
                    input["resource_changes"][source]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                5 => input["resource_changes"][source]["change"]["after_unknown"] = json!(true),
                6 => input["resource_changes"][source]["change"]["after_sensitive"] = json!(true),
                7 => input["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                8 => {
                    input["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other")
                }
                _ => input["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            }
            assert!(
                !complete(&input, "aws_lambda_function.app", field),
                "{field} {case}"
            );
        }
    }
}

#[test]
fn lambda_config_signer_requires_complete_version_collection() {
    for case in 0..8 {
        let mut input = fixture();
        match case {
            0 => {
                input["resource_changes"][2]["change"]["after"]["allowed_publishers"][0]["signing_profile_version_arns"] =
                    json!(["PRIVATE_PROFILE_VERSION", "literal"])
            }
            1 => {
                input["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"allowed_publishers":[{"signing_profile_version_arns":[true]}]})
            }
            2 => {
                input["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"allowed_publishers":[{},{}]})
            }
            3 => {
                input["resource_changes"][2]["change"]["after"]["allowed_publishers"][0]["signing_profile_version_arns"] =
                    json!([])
            }
            4 => {
                input["configuration"]["root_module"]["resources"][2]["expressions"]["allowed_publishers"]
                    [0]["signing_profile_version_arns"] =
                    json!({"references":["aws_signer_signing_profile.app.arn"]})
            }
            5 => {
                input["configuration"]["root_module"]["resources"][2]["expressions"]["allowed_publishers"]
                    [0]["signing_profile_version_arns"] =
                    json!({"constant_value":["PRIVATE_PROFILE_VERSION"]})
            }
            6 => {
                input["resource_changes"][8]["change"]["after"]["version_arn"] =
                    json!("different-version")
            }
            _ => {
                input["resource_changes"][2]["change"]["after"]["allowed_publishers"][0]["signing_profile_version_arns"] =
                    json!(["PRIVATE_PROFILE_VERSION", "PRIVATE_PROFILE_VERSION"])
            }
        }
        assert!(
            !complete(
                &input,
                "aws_lambda_code_signing_config.app",
                "allowed_publishers.signing_profile_version_arns"
            ),
            "{case}"
        );
    }
}

#[test]
fn lambda_config_network_proof_rejects_unknown_and_incomplete_collections() {
    for case in 0..6 {
        let mut input = fixture();
        match case {
            0 => {
                input["resource_changes"][0]["change"]["after_unknown"] = json!({"vpc_config":true})
            }
            1 => {
                input["resource_changes"][0]["change"]["after"]["vpc_config"][0]["subnet_ids"] =
                    json!(["subnet-a", "literal"])
            }
            2 => input["resource_changes"][10]["change"]["after"]["vpc_id"] = json!("wrong"),
            3 => input["resource_changes"][10]["change"]["after_unknown"] = json!({"id":true}),
            4 => input["resource_changes"][10]["mode"] = json!("data"),
            _ => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(
            !graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
                && graph.nodes[e.to].address == "aws_lambda_function.app"),
            "{case}"
        );
    }
}

#[test]
fn lambda_config_named_states_and_selection_preserve_identity() {
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture().to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        let graph = semantic::transform(raw);
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_lambda_function.app".into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!selected.nodes.is_empty());
    }
    let mut raw = plan::parse(&fixture().to_string()).unwrap();
    for node in &mut raw.nodes {
        node.provider_configuration.as_mut().unwrap().account = Some(
            crate::model::AwsAccountScope::new(if node.resource_type == "aws_iam_role" {
                "222222222222"
            } else {
                "111111111111"
            })
            .unwrap(),
        );
    }
    assert!(
        !semantic::transform(&raw)
            .nodes
            .iter()
            .find(|n| n.address == "aws_lambda_function.app")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Execution role reference" && r.complete)
    );
}

#[test]
fn lambda_config_alternative_types_and_indexed_complete_signers() {
    for (from, to) in [
        ("aws_sqs_queue", "aws_sns_topic"),
        ("aws_efs_access_point", "aws_s3files_access_point"),
    ] {
        let input: Value = serde_json::from_str(&fixture().to_string().replace(from, to)).unwrap();
        let field = if from == "aws_sqs_queue" {
            "dead_letter_config.target_arn"
        } else {
            "file_system_config.arn"
        };
        assert!(complete(&input, "aws_lambda_function.app", field));
    }
    let mut input = fixture();
    input["resource_changes"][8]["address"] = json!("aws_signer_signing_profile.app[0]");
    input["resource_changes"][8]["index"] = json!(0);
    let mut other = input["resource_changes"][8].clone();
    other["address"] = json!("aws_signer_signing_profile.app[1]");
    other["index"] = json!(1);
    other["change"]["after"]["version_arn"] = json!("PRIVATE_SECOND_VERSION");
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other);
    input["resource_changes"][2]["change"]["after"]["allowed_publishers"][0]["signing_profile_version_arns"] =
        json!(["PRIVATE_SECOND_VERSION", "PRIVATE_PROFILE_VERSION"]);
    for (references, valid) in [
        (
            json!([
                "aws_signer_signing_profile.app[1].version_arn",
                "aws_signer_signing_profile.app[0].version_arn"
            ]),
            true,
        ),
        (json!(["aws_signer_signing_profile.app.version_arn"]), false),
        (
            json!(["aws_signer_signing_profile.app[0].version_arn"]),
            false,
        ),
    ] {
        input["configuration"]["root_module"]["resources"][2]["expressions"]["allowed_publishers"]
            [0]["signing_profile_version_arns"] = json!({"references":references});
        assert_eq!(
            complete(
                &input,
                "aws_lambda_code_signing_config.app",
                "allowed_publishers.signing_profile_version_arns"
            ),
            valid
        );
    }
}
