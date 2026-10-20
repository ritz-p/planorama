use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/s3-notification-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn reference<'a>(result: &'a Graph, attribute: &str) -> &'a ResourceReference {
    result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_notification.assets")
        .unwrap()
        .references
        .iter()
        .find(|r| r.attribute == attribute && r.label != "References")
        .unwrap()
}

#[test]
fn s3_notifications_keep_distinct_list_provenance_and_original_resources() {
    let value = fixture();
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let result = semantic::transform(&parsed);
    assert_eq!(result.coverage.selected.resources, 7);
    assert_eq!(result.nodes.len(), 7);
    for (attribute, address) in [
        ("bucket", "aws_s3_bucket.assets[0]"),
        ("queue.0.queue_arn", "aws_sqs_queue.events[0]"),
        ("queue.1.queue_arn", "aws_sqs_queue.events[1]"),
        ("topic.0.topic_arn", "aws_sns_topic.events"),
        (
            "lambda_function.0.lambda_function_arn",
            "aws_lambda_function.events",
        ),
    ] {
        let r = reference(&result, attribute);
        assert!(r.complete, "{attribute}");
        assert_eq!(r.targets[0].id.address, address);
    }
    assert_eq!(
        parsed
            .attributes
            .iter()
            .find(|r| r.attribute == "queue.1.queue_arn")
            .unwrap()
            .indexed_block_lengths,
        vec![2]
    );
    assert!(
        !result
            .nodes
            .iter()
            .find(|n| n.address == "aws_s3_bucket_notification.literal")
            .unwrap()
            .references
            .iter()
            .find(|r| r.label == "Notification queue reference")
            .unwrap()
            .complete
    );
    assert!(result.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    let notification = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_notification.assets")
        .unwrap();
    assert_eq!(notification.placement, Placement::Logical);
    assert_eq!(notification.role, ResourceRole::Controller);
    assert_eq!(notification.action, Action::Update);
    assert!(
        !notification
            .references
            .iter()
            .any(|r| r.attribute == "queue.queue_arn")
    );
}

#[test]
fn s3_notifications_do_not_mix_entries_or_accept_changed_list_shapes() {
    for case in 0..7 {
        let mut value = fixture();
        match case {
            0 => {
                value["resource_changes"][5]["change"]["after"]["queue"]
                    .as_array_mut()
                    .unwrap()
                    .reverse();
            }
            1 => {
                value["resource_changes"][5]["change"]["after"]["queue"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            2 => value["resource_changes"][5]["change"]["after_unknown"] = json!({"queue":true}),
            3 => {
                value["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"queue":[{"queue_arn":true},{}]})
            }
            4 => value["resource_changes"][5]["change"]["after_unknown"] = json!({"queue":[{}]}),
            5 => {
                value["resource_changes"][5]["change"]["after"]["queue"] =
                    json!({"0":{"queue_arn":"arn:aws:sqs:us-west-2:123456789012:events-a"}})
            }
            _ => {
                value["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"queue":{"0":{"queue_arn":true}}})
            }
        }
        let result = graph(&value);
        assert!(
            !reference(&result, "queue.0.queue_arn").complete,
            "case {case}"
        );
        if case == 3 {
            assert!(reference(&result, "queue.1.queue_arn").complete);
        } else {
            assert!(!reference(&result, "queue.1.queue_arn").complete);
        }
        assert!(reference(&result, "topic.0.topic_arn").complete);
    }
    let mut value = fixture();
    value["resource_changes"][5]["change"]["after"]["queue"][1]["queue_arn"] =
        value["resource_changes"][1]["change"]["after"]["arn"].clone();
    value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"][1]["queue_arn"] =
        json!({"references":["aws_sqs_queue.events[0].arn"]});
    let result = graph(&value);
    assert!(reference(&result, "queue.0.queue_arn").complete);
    assert!(reference(&result, "queue.1.queue_arn").complete);
    assert_eq!(
        reference(&result, "queue.0.queue_arn").targets,
        reference(&result, "queue.1.queue_arn").targets
    );
}

#[test]
fn s3_notification_endpoints_require_current_managed_explicit_typed_references() {
    for case in 0..10 {
        let mut value = fixture();
        match case {
            0 => {
                value["resource_changes"][1]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            1 => value["resource_changes"][1]["type"] = json!("aws_sns_topic"),
            2 => value["resource_changes"][1]["mode"] = json!("data"),
            3 => value["resource_changes"][1]["deposed"] = json!("old"),
            4 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            5 => value["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            6 => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"][0]["queue_arn"] =
                    json!({"references":["aws_sqs_queue.events[count.index].arn"]})
            }
            7 => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"][0]["queue_arn"] =
                    json!({"references":["aws_sqs_queue.events.arn"]})
            }
            8 => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"][0]["queue_arn"] =
                    json!({"references":["aws_sqs_queue.events[0].arn","var.missing"]})
            }
            _ => {
                value["resource_changes"].as_array_mut().unwrap().remove(1);
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let result = semantic::transform(&parsed);
        assert!(
            !reference(&result, "queue.0.queue_arn").complete,
            "case {case}"
        );
        assert!(
            semantic::diagnostics::collect(&parsed)
                .iter()
                .any(|d| d.attribute == "queue.0.queue_arn"
                    && d.reason == DiagnosticReason::UnprovenEndpointValue)
        );
    }
}

#[test]
fn s3_notifications_require_list_configuration_for_positional_proof() {
    let mut value = fixture();
    value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"] =
        value["configuration"]["root_module"]["resources"][4]["expressions"]["queue"][0].clone();
    value["resource_changes"][5]["change"]["after"]["queue"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let result = graph(&value);
    assert!(!reference(&result, "queue.queue_arn").complete);
}

#[test]
fn s3_notification_lifecycle_selection_and_exports_keep_values_private() {
    let mut value = fixture();
    value["resource_changes"][5]["previous_address"] = json!("aws_s3_bucket_notification.old");
    value["resource_changes"][5]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][5].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let result = graph(&value);
    assert_eq!(result.coverage.selected.resources, 8);
    assert!(
        result
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
        assert!(output.contains("Notification queue reference"));
    }
    let selected = crate::view::single(
        &graph(&fixture()),
        &crate::view::Options {
            focus: Some("aws_s3_bucket_notification.assets".into()),
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
    let mut partial = fixture();
    partial["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let states = plan::parse_inputs(
        [("one", fixture()), ("two", partial)]
            .into_iter()
            .map(|(name, value)| PlanInput {
                state_id: StateId::new(name).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, parsed) in states.states {
        assert_eq!(
            reference(&semantic::transform(&parsed), "queue.0.queue_arn").complete,
            state.as_str() == "one"
        );
    }
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"]["module_calls"] =
        json!({"storage":{"module":{"resources":resources}}});
    for node in value["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!(
            "module.storage.{}",
            node["address"].as_str().unwrap()
        ));
        node["module_address"] = json!("module.storage");
    }
    let output = crate::generate(
        &value.to_string(),
        &crate::Options {
            modules: vec!["module.storage".into()],
            icon_mode: crate::icons::IconMode::ServiceName,
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("8 resources"));
    assert!(output.contains("Amazon S3"));
    assert!(!output.contains("PRIVATE_"));
}
