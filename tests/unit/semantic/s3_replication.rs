use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/s3-replication-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn reference<'a>(result: &'a Graph, attribute: &str) -> &'a ResourceReference {
    result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_replication_configuration.assets")
        .unwrap()
        .references
        .iter()
        .find(|r| r.attribute == attribute && r.label != "References")
        .unwrap()
}

#[test]
fn s3_replication_preserves_source_and_individual_destination_provenance() {
    let parsed = plan::parse(&fixture().to_string()).unwrap();
    let mut reordered = fixture();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(parsed, plan::parse(&reordered.to_string()).unwrap());
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let result = semantic::transform(&parsed);
    assert_eq!(result.nodes.len(), 6);
    assert_eq!(result.coverage.selected.resources, 6);
    assert!(reference(&result, "bucket").complete);
    for i in 0..2 {
        let path = format!("rule.{i}.destination.0.bucket");
        let r = reference(&result, &path);
        assert!(r.complete);
        assert_eq!(
            r.targets[0].id.address,
            format!("aws_s3_bucket.destination[{i}]")
        );
        assert_eq!(
            parsed
                .attributes
                .iter()
                .find(|r| r.attribute == path)
                .unwrap()
                .indexed_block_lengths,
            vec![2, 1]
        );
        let destination = result
            .nodes
            .iter()
            .find(|n| n.address == r.targets[0].id.address)
            .unwrap();
        assert_eq!(
            destination
                .entity
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
    let node = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_s3_bucket_replication_configuration.assets")
        .unwrap();
    assert_eq!(node.role, ResourceRole::Controller);
    assert_eq!(node.placement, Placement::Logical);
    assert_eq!(
        node.entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "us-west-2"
    );
    assert!(result.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    assert!(
        !result
            .nodes
            .iter()
            .find(|n| n.address == "aws_s3_bucket_replication_configuration.literal")
            .unwrap()
            .references
            .iter()
            .find(|r| r.label == "Replication destination reference")
            .unwrap()
            .complete
    );
}

#[test]
fn s3_replication_does_not_borrow_proof_across_rules_or_nested_destinations() {
    for case in 0..7 {
        let mut value = fixture();
        match case {
            0 => value["resource_changes"][4]["change"]["after"]["rule"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            1 => {
                value["resource_changes"][4]["change"]["after"]["rule"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            2 => {
                let destination =
                    value["resource_changes"][4]["change"]["after"]["rule"][0]["destination"][0]
                        .clone();
                value["resource_changes"][4]["change"]["after"]["rule"][0]["destination"]
                    .as_array_mut()
                    .unwrap()
                    .push(destination);
            }
            3 => {
                value["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"rule":[{"destination":true},{}]})
            }
            4 => {
                value["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"rule":[{"destination":[{"bucket":true}]},{}]})
            }
            5 => {
                value["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"rule":[{"destination":[]},{}]})
            }
            _ => value["resource_changes"][4]["change"]["after_unknown"] = json!({"rule":true}),
        }
        let result = graph(&value);
        assert!(
            !reference(&result, "rule.0.destination.0.bucket").complete,
            "case {case}"
        );
        assert_eq!(
            reference(&result, "rule.1.destination.0.bucket").complete,
            (2..6).contains(&case),
            "case {case}"
        );
    }
}

#[test]
fn s3_replication_rejects_multiple_destinations_in_one_rule() {
    let mut value = fixture();
    let destination =
        value["resource_changes"][4]["change"]["after"]["rule"][0]["destination"][0].clone();
    value["resource_changes"][4]["change"]["after"]["rule"][0]["destination"]
        .as_array_mut()
        .unwrap()
        .push(destination);
    let expression = value["configuration"]["root_module"]["resources"][3]["expressions"]["rule"]
        [0]["destination"][0]
        .clone();
    value["configuration"]["root_module"]["resources"][3]["expressions"]["rule"][0]["destination"]
        .as_array_mut()
        .unwrap()
        .push(expression);
    let result = graph(&value);
    assert!(!reference(&result, "rule.0.destination.0.bucket").complete);
    assert!(!reference(&result, "rule.0.destination.1.bucket").complete);
}

#[test]
fn s3_replication_requires_typed_current_explicit_and_unique_bucket_endpoints() {
    for case in 0..12 {
        let mut value = fixture();
        match case {
            0 => {
                value["resource_changes"][1]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            1 => value["resource_changes"][1]["type"] = json!("aws_sqs_queue"),
            2 => value["resource_changes"][1]["mode"] = json!("data"),
            3 => value["resource_changes"][1]["deposed"] = json!("old"),
            4 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            5 => value["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            6 => {
                value["configuration"]["root_module"]["resources"][3]["expressions"]["rule"][0]["destination"]
                    [0]["bucket"] =
                    json!({"references":["aws_s3_bucket.destination[count.index].arn"]})
            }
            7 => {
                value["configuration"]["root_module"]["resources"][3]["expressions"]["rule"][0]["destination"]
                    [0]["bucket"] = json!({"references":["aws_s3_bucket.destination.arn"]})
            }
            8 => {
                value["configuration"]["root_module"]["resources"][3]["expressions"]["rule"][0]["destination"]
                    [0]["bucket"] =
                    json!({"references":["aws_s3_bucket.destination[0].arn","var.missing"]})
            }
            9 => value["resource_changes"][1]["change"]["after"]["arn"] = json!("different"),
            10 => value["resource_changes"][1]["change"]["after_unknown"] = json!({"arn":true}),
            _ => {
                value["resource_changes"].as_array_mut().unwrap().remove(1);
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            !reference(&semantic::transform(&parsed), "rule.0.destination.0.bucket").complete,
            "case {case}"
        );
        assert!(
            semantic::diagnostics::collect(&parsed)
                .iter()
                .any(|d| d.attribute == "rule.0.destination.0.bucket"
                    && d.reason == DiagnosticReason::UnprovenEndpointValue)
        );
    }
}

#[test]
fn s3_replication_lifecycle_selection_states_and_output_privacy() {
    let mut value = fixture();
    value["resource_changes"][4]["previous_address"] =
        json!("aws_s3_bucket_replication_configuration.old");
    value["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    value["resource_changes"][4]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = value["resource_changes"][4].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let result = graph(&value);
    assert_eq!(result.coverage.selected.resources, 7);
    let current = result
        .nodes
        .iter()
        .find(|n| {
            n.address == "aws_s3_bucket_replication_configuration.assets" && n.deposed_key.is_none()
        })
        .unwrap();
    assert_eq!(current.action, Action::Replace);
    assert!(current.previous_address.is_some() && current.metadata.import.is_some());
    assert!(
        result
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        for icon_mode in [
            crate::icons::IconMode::Icon,
            crate::icons::IconMode::ServiceName,
        ] {
            let output = crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    icon_mode,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!output.contains("PRIVATE_"));
            assert!(!output.contains("arn:aws:"));
            assert!(output.contains("Replication destination reference"));
        }
    }
    let selected = crate::view::single(
        &graph(&fixture()),
        &crate::view::Options {
            focus: Some("aws_s3_bucket_replication_configuration.assets".into()),
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
            reference(&semantic::transform(&parsed), "rule.0.destination.0.bucket").complete,
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
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("7 resources"));
    assert!(!output.contains("PRIVATE_"));
}
