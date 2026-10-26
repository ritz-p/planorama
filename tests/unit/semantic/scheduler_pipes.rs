use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/eventbridge-configuration-plan.json");

fn complete(value: &Value, index: usize, field: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let address = value["resource_changes"][index]["address"]
        .as_str()
        .unwrap();
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && super::label(
                    value["resource_changes"][index]["type"].as_str().unwrap(),
                    field,
                ) == Some(r.label)
        })
}

fn set_path(value: &mut Value, path: &[&str], replacement: Value) {
    if value.is_array() {
        set_path(&mut value[0], path, replacement);
    } else if let Some((field, rest)) = path.split_first() {
        set_path(&mut value[*field], rest, replacement);
    } else {
        *value = replacement;
    }
}

#[test]
fn references_require_known_unique_current_compatible_endpoints() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (index, field, endpoint) in [
        (5, "group_name", 0),
        (5, "target.arn", 4),
        (5, "target.role_arn", 1),
        (6, "source", 2),
        (6, "enrichment", 3),
        (6, "target", 4),
        (6, "role_arn", 1),
    ] {
        assert!(complete(&fixture, index, field), "{field}");
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "ambiguous",
            "wrong-type",
            "wrong-provider",
            "deleted",
            "consumer-deleted",
            "deposed",
            "data",
            "region",
            "unresolved-region",
            "mismatch",
            "wrong-field",
            "mask-shape",
        ] {
            let mut v = fixture.clone();
            let path = field.split('.').collect::<Vec<_>>();
            let address = v["resource_changes"][endpoint]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                "literal" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"constant_value":"PRIVATE_VALUE"}),
                ),
                "unknown" => {
                    v["resource_changes"][index]["change"]["after_unknown"] = json!(true);
                }
                "source-unknown" => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!(true);
                }
                "ambiguous" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":[format!("{address}.arn"),"var.unknown"]}),
                ),
                "wrong-type" => v["resource_changes"][endpoint]["type"] = json!("aws_s3_bucket"),
                "wrong-provider" => {
                    v["resource_changes"][endpoint]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "deleted" => {
                    v["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"])
                }
                "consumer-deleted" => {
                    v["resource_changes"][index]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][endpoint]["deposed"] = json!("old"),
                "data" => v["resource_changes"][endpoint]["mode"] = json!("data"),
                "region" => {
                    let mut config = v["configuration"]["provider_config"]["aws"].clone();
                    config["expressions"]["region"] = json!({"constant_value":"us-west-2"});
                    v["configuration"]["provider_config"]["aws.other"] = config;
                    v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                        json!("aws.other");
                }
                "unresolved-region" => {
                    v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                "mismatch" => set_path(
                    &mut v["resource_changes"][index]["change"]["after"],
                    &path,
                    json!("PRIVATE_OTHER"),
                ),
                "wrong-field" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":[format!("{address}.wrong")]}),
                ),
                "mask-shape" => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!([false])
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, index, field), "{field}: {case}");
        }
    }
}

#[test]
fn source_discriminator_and_nested_unknowns_are_conservative() {
    for mask in [
        json!(true),
        json!([true]),
        json!([{"rabbitmq_broker_parameters":true}]),
        json!({"bad":false}),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][6]["change"]["after_unknown"] = json!({"source_parameters":mask});
        assert!(!complete(&v, 6, "source"));
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][6]["change"]["after"]["source_parameters"][0]["kinesis_stream_parameters"] =
        json!([{"starting_position":"LATEST"}]);
    assert!(!complete(&v, 6, "source"));
    for mask in [
        json!([{"arn":true}]),
        json!([false, false]),
        json!({"arn":false}),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][5]["change"]["after_unknown"] = json!({"target":mask});
        assert!(!complete(&v, 5, "target.arn"));
    }
}

#[test]
fn allowed_endpoint_roles_remain_separate() {
    for kind in [
        "aws_sqs_queue",
        "aws_sns_topic",
        "aws_lambda_function",
        "aws_kinesis_stream",
        "aws_sfn_state_machine",
        "aws_cloudwatch_event_bus",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][4]["type"] = json!(kind);
        assert!(complete(&v, 5, "target.arn"), "{kind}");
        assert!(complete(&v, 6, "target"), "{kind}");
        v["resource_changes"][3]["type"] = json!(kind);
        assert_eq!(complete(&v, 6, "enrichment"), kind == "aws_lambda_function");
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][2]["type"] = json!("aws_kinesis_stream");
    v["resource_changes"][6]["change"]["after"]["source_parameters"] =
        json!([{"kinesis_stream_parameters":[{"starting_position":"LATEST"}]}]);
    assert!(complete(&v, 6, "source"));
}

#[test]
fn presentation_preserves_identity_filtering_states_and_safe_output() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][5]["previous_address"] = json!("aws_scheduler_schedule.old");
    v["resource_changes"][5]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][5]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let parsed = plan::parse(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 8);
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    for source in &parsed.nodes {
        let nodes = graph
            .nodes
            .iter()
            .filter(|n| n.address == source.address)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
        assert!(nodes[0].member_owner.is_none());
        if source.resource_type == "aws_pipes_pipe" {
            assert_eq!(nodes[0].placement, Placement::Spatial);
        }
        if source.resource_type == "aws_scheduler_schedule" {
            assert_eq!(nodes[0].placement, Placement::Logical);
        }
    }
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_scheduler_schedule.daily".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(out.contains("Schedule group membership"));
        assert!(out.contains("Pipe enrichment"));
    }
    let diagnostics = crate::provider::diagnostics(&parsed);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.address == "aws_scheduler_schedule.literal")
    );
}
