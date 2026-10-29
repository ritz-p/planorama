use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/lambda-events-plan.json")).unwrap()
}

fn change<'a>(input: &'a mut Value, address: &str) -> &'a mut Value {
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()
}

fn expressions<'a>(input: &'a mut Value, address: &str) -> &'a mut Value {
    &mut input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()["expressions"]
}

fn edge(input: &Value, name: &str) -> bool {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
        .edges
        .iter()
        .any(|e| {
            e.kind == EdgeKind::EventSource
                && e.change.as_ref().unwrap().address
                    == format!("aws_lambda_event_source_mapping.{name}")
        })
}

fn reference(input: &Value, name: &str, field: &str) -> bool {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == format!("aws_lambda_event_source_mapping.{name}"))
        .unwrap()
        .references
        .iter()
        .any(|r| {
            !r.dependency
                && r.attribute == field
                && r.complete
                && super::label("aws_lambda_event_source_mapping", field) == Some(r.label)
        })
}

#[test]
fn lambda_events_typed_sources_and_qualified_functions_keep_independent_identity() {
    let input = fixture();
    for name in ["ddb", "msk", "serverless", "mq", "docdb"] {
        assert!(edge(&input, name), "{name}");
    }
    assert!(!edge(&input, "self"));
    for name in ["ddb", "msk", "serverless", "self"] {
        assert!(
            reference(
                &input,
                name,
                "destination_config.on_failure.destination_arn"
            ),
            "{name}"
        );
    }
    for name in ["mq", "docdb", "self"] {
        assert!(
            reference(&input, name, "source_access_configuration.uri"),
            "{name}"
        );
    }
    assert!(reference(
        &input,
        "msk",
        "amazon_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri"
    ));
    assert!(reference(
        &input,
        "self",
        "self_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri"
    ));
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::EventSource
                && graph.nodes[e.to].resource_type == "aws_lambda_function")
            .count(),
        5
    );
    assert_eq!(
        graph.coverage.selected.resources,
        input["resource_changes"].as_array().unwrap().len()
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| matches!(e.kind, EdgeKind::Containment | EdgeKind::Connection))
    );
    assert!(!format!("{raw:?}{graph:?}").contains("kafka.invalid"));
    let mut reordered = input;
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        graph,
        semantic::transform(&plan::parse(&reordered.to_string()).unwrap())
    );
}

#[test]
fn lambda_events_reject_wrong_output_qualifier_parent_and_source_context() {
    for case in 0..10 {
        let mut input = fixture();
        match case {
            0 => {
                expressions(&mut input, "aws_lambda_event_source_mapping.ddb")["event_source_arn"] =
                    json!({"references":["aws_dynamodb_table.app.arn"]})
            }
            1 => {
                change(&mut input, "aws_dynamodb_table.app")["change"]["after"]["stream_arn"] =
                    json!("different")
            }
            2 => {
                change(&mut input, "aws_lambda_function.app")["change"]["after"]["version"] =
                    json!("$LATEST")
            }
            3 => {
                change(&mut input, "aws_lambda_function.app")["change"]["after_unknown"] =
                    json!({"qualified_arn":true})
            }
            4 => {
                change(&mut input, "aws_lambda_function.app")["change"]["after"]["version"] =
                    json!("4")
            }
            5 => {
                change(&mut input, "aws_lambda_event_source_mapping.ddb")["change"]["after"]["self_managed_event_source"] =
                    json!([{"endpoints":{}}])
            }
            6 => {
                change(&mut input, "aws_lambda_event_source_mapping.ddb")["change"]["after"]["document_db_event_source_config"] =
                    json!([{}])
            }
            7 => {
                expressions(&mut input, "aws_lambda_event_source_mapping.ddb")["function_name"] =
                    json!({"references":["aws_lambda_function.app.qualified_arn","var.unknown"]})
            }
            8 => {
                expressions(&mut input, "aws_lambda_event_source_mapping.ddb")["event_source_arn"] = json!({"constant_value":"arn:aws:dynamodb:us-east-1:123456789012:table/app/stream/2026-10-01T00:00:00.000"})
            }
            _ => {
                change(&mut input, "aws_dynamodb_table.app")["change"]["after_sensitive"] =
                    json!({"stream_arn":true})
            }
        }
        assert!(!edge(&input, "ddb"), "{case}");
    }
    for case in 0..3 {
        let mut input = fixture();
        match case {
            0 => {
                change(&mut input, "aws_lambda_alias.live")["change"]["after"]["name"] =
                    json!("other")
            }
            1 => {
                expressions(&mut input, "aws_lambda_alias.live")["function_name"] =
                    json!({"constant_value":"app"})
            }
            _ => {
                change(&mut input, "aws_lambda_function.app")["change"]["after"]["arn"] =
                    json!("other-parent")
            }
        }
        assert!(!edge(&input, "msk"));
    }
}

#[test]
fn lambda_events_new_proofs_reject_data_deleted_deposed_foreign_and_unknown_scope() {
    for name in ["ddb", "msk", "serverless", "mq", "docdb"] {
        for case in 0..7 {
            let mut input = fixture();
            let address = format!("aws_lambda_event_source_mapping.{name}");
            match case {
                0 => change(&mut input, &address)["mode"] = json!("data"),
                1 => change(&mut input, &address)["change"]["actions"] = json!(["delete"]),
                2 => change(&mut input, &address)["deposed"] = json!("old"),
                3 => {
                    change(&mut input, "aws_lambda_function.app")["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                4 => change(&mut input, "aws_lambda_alias.live")["mode"] = json!("data"),
                5 => {
                    input["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                _ => change(&mut input, "aws_lambda_function.app")["deposed"] = json!("old"),
            }
            if case == 4 && name == "ddb" {
                continue;
            }
            assert!(!edge(&input, name), "{name} {case}");
        }
    }
}

#[test]
fn lambda_events_access_sets_require_typed_exact_per_instance_outputs() {
    for case in 0..11 {
        let mut input = fixture();
        match case {
            0 => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"][0]["type"] = json!("VIRTUAL_HOST"),
            1 => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after_unknown"] = json!({"source_access_configuration":[{"type":true},{},{}]}),
            2 => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after_unknown"] = json!({"source_access_configuration":[{}]}),
            3 => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"][1]["uri"] = json!("subnet-access"),
            4 => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"].as_array_mut().unwrap().push(json!({"type":"BASIC_AUTH","uri":"literal"})),
            5 => expressions(&mut input, "aws_lambda_event_source_mapping.self")["source_access_configuration"][1]["uri"] = json!({"constant_value":"subnet:subnet-access"}),
            6 => expressions(&mut input, "aws_lambda_event_source_mapping.self")["source_access_configuration"][0]["uri"] = json!({"references":["aws_secretsmanager_secret.auth.id"]}),
            7 => change(&mut input, "aws_subnet.access")["mode"] = json!("data"),
            8 => change(&mut input, "aws_security_group.access")["change"]["after_sensitive"] = json!({"id":true}),
            9 => { let entry = change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"][0].clone(); change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"].as_array_mut().unwrap().push(entry); },
            _ => change(&mut input, "aws_lambda_event_source_mapping.self")["change"]["after"]["source_access_configuration"] = json!([]),
        }
        assert!(
            !reference(&input, "self", "source_access_configuration.uri"),
            "{case}"
        );
    }
    let input: Value = serde_json::from_str(
        &fixture()
            .to_string()
            .replace(
                "aws_secretsmanager_secret.auth",
                "aws_secretsmanager_secret_version.auth",
            )
            .replace(
                "\"aws_secretsmanager_secret\"",
                "\"aws_secretsmanager_secret_version\"",
            ),
    )
    .unwrap();
    assert!(reference(&input, "self", "source_access_configuration.uri"));
}

#[test]
fn lambda_events_failure_and_registry_references_require_source_type_and_shape() {
    for case in 0..5 {
        let mut input = fixture();
        match case {
            0 => change(&mut input, "aws_sqs_queue.failed")["type"] = json!("aws_lambda_function"),
            1 => {
                change(&mut input, "aws_lambda_event_source_mapping.ddb")["change"]["after_unknown"] =
                    json!({"destination_config":true})
            }
            2 => {
                change(&mut input, "aws_lambda_event_source_mapping.ddb")["change"]["after"]["destination_config"] =
                    json!([])
            }
            3 => change(&mut input, "aws_dynamodb_table.app")["type"] = json!("aws_s3_bucket"),
            _ => {
                expressions(&mut input, "aws_lambda_event_source_mapping.ddb")["destination_config"]
                    [0]["on_failure"][0]["destination_arn"] =
                    json!({"references":["aws_sqs_queue.failed.name"]})
            }
        }
        assert!(
            !reference(
                &input,
                "ddb",
                "destination_config.on_failure.destination_arn"
            ),
            "{case}"
        );
    }
    for case in 0..4 {
        let mut input = fixture();
        match case {
            0 => change(&mut input, "aws_glue_registry.app")["type"] = json!("aws_glue_schema"),
            1 => {
                change(&mut input, "aws_lambda_event_source_mapping.msk")["change"]["after_unknown"] = json!({"amazon_managed_kafka_event_source_config":[{"schema_registry_config":true}]})
            }
            2 => {
                change(&mut input, "aws_lambda_event_source_mapping.msk")["change"]["after"]["amazon_managed_kafka_event_source_config"]
                    [0]["schema_registry_config"][0]["schema_registry_uri"] =
                    json!("https://registry.invalid")
            }
            _ => change(&mut input, "aws_msk_cluster.app")["type"] = json!("aws_mq_broker"),
        }
        assert!(
            !reference(
                &input,
                "msk",
                "amazon_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri"
            ),
            "{case}"
        );
    }
}

#[test]
fn lambda_events_preserve_actions_named_states_filtering_and_scope() {
    let mut input = fixture();
    change(&mut input, "aws_lambda_event_source_mapping.ddb")["change"]["actions"] =
        json!(["delete", "create"]);
    change(&mut input, "aws_lambda_event_source_mapping.ddb")["previous_address"] =
        json!("aws_lambda_event_source_mapping.old");
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: input.to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        let graph = semantic::transform(raw);
        let changed = graph
            .edges
            .iter()
            .find(|e| {
                e.change
                    .as_ref()
                    .is_some_and(|c| c.address.ends_with(".ddb"))
            })
            .unwrap()
            .change
            .as_ref()
            .unwrap();
        assert_eq!(changed.action, crate::model::Action::Replace);
        assert_eq!(
            changed.previous_address.as_deref(),
            Some("aws_lambda_event_source_mapping.old")
        );
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_lambda_event_source_mapping.ddb".into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::EventSource)
        );
    }
    let mut raw = plan::parse(&fixture().to_string()).unwrap();
    for node in &mut raw.nodes {
        node.provider_configuration.as_mut().unwrap().account = Some(
            crate::model::AwsAccountScope::new(if node.resource_type == "aws_dynamodb_table" {
                "222222222222"
            } else {
                "111111111111"
            })
            .unwrap(),
        );
    }
    assert!(
        !semantic::transform(&raw)
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::EventSource
                && e.change.as_ref().unwrap().address.ends_with(".ddb"))
    );
}
