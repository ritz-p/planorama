use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

const SOURCE: &str = "kinesis_source_configuration.kinesis_stream_arn";
const DELIVERY: &str = "extended_s3_configuration.bucket_arn";
const BACKUP: &str = "extended_s3_configuration.s3_backup_configuration.bucket_arn";

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/firehose-plan.json")).unwrap()
}

fn verified(value: &Value, attribute: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), parsed.nodes.len());
    assert_eq!(graph.edges.len(), parsed.edges.len());
    let reference = graph
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".archive"))
        .unwrap()
        .references
        .iter()
        .find(|r| r.attribute == attribute && r.label != "References");
    let complete = reference.is_some_and(|r| r.complete);
    if !complete {
        assert!(semantic::diagnostics::collect(&parsed).iter().any(|d| {
            d.address.ends_with(".archive")
                && d.attribute == attribute
                && d.reason == DiagnosticReason::UnprovenEndpointValue
        }));
    }
    complete
}

#[test]
fn firehose_separates_source_delivery_and_enabled_backup_references() {
    let input = fixture();
    for attribute in [SOURCE, DELIVERY, BACKUP] {
        assert!(verified(&input, attribute));
    }
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(graph.coverage.input.resources, 5);
    assert_eq!(graph.coverage.selected.resources, 5);
    let stream = graph
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".archive"))
        .unwrap();
    assert_eq!(stream.role, ResourceRole::Node);
    assert_eq!(stream.placement, Placement::Spatial);
    assert_eq!(
        stream
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
    for (attribute, label, address) in [
        (SOURCE, "Kinesis source", "aws_kinesis_stream.input[0]"),
        (
            DELIVERY,
            "S3 delivery destination",
            "aws_s3_bucket.delivery",
        ),
        (BACKUP, "S3 backup destination", "aws_s3_bucket.backup"),
    ] {
        let reference = stream
            .references
            .iter()
            .find(|r| r.attribute == attribute)
            .unwrap();
        assert_eq!(reference.label, label);
        assert_eq!(reference.targets[0].id.address, address);
    }
    for case in ["disabled", "unknown", "missing", "other"] {
        let mut input = fixture();
        input["resource_changes"][3]["change"]["after"]["extended_s3_configuration"][0]["s3_backup_mode"] =
            match case {
                "disabled" => json!("Disabled"),
                "other" => json!("AllData"),
                _ => Value::Null,
            };
        if case == "unknown" {
            input["resource_changes"][3]["change"]["after_unknown"] =
                json!({"extended_s3_configuration": [{"s3_backup_mode": true}]});
        }
        assert!(!verified(&input, BACKUP));
        assert!(verified(&input, DELIVERY));
    }
    for unknown in [false, true] {
        let mut input = fixture();
        if unknown {
            input["resource_changes"][3]["change"]["after_unknown"] =
                json!({"extended_s3_configuration": [{"s3_backup_configuration": true}]});
        } else {
            let backup = &mut input["resource_changes"][3]["change"]["after"]["extended_s3_configuration"]
                [0]["s3_backup_configuration"];
            let duplicate = backup[0].clone();
            backup.as_array_mut().unwrap().push(duplicate);
        }
        assert!(!verified(&input, BACKUP));
        assert!(verified(&input, DELIVERY));
    }
    let mut shared_bucket = fixture();
    shared_bucket["resource_changes"][3]["change"]["after"]["extended_s3_configuration"][0]["s3_backup_configuration"]
        [0]["bucket_arn"] = json!("arn:aws:s3:::fixture-delivery");
    shared_bucket["configuration"]["root_module"]["resources"][3]["expressions"]["extended_s3_configuration"]
        [0]["s3_backup_configuration"][0]["bucket_arn"] =
        json!({"references": ["aws_s3_bucket.delivery.arn"]});
    assert!(verified(&shared_bucket, DELIVERY));
    assert!(verified(&shared_bucket, BACKUP));
}

#[test]
fn firehose_rejects_unproven_modes_and_ambiguous_configuration_blocks() {
    for case in 0..12 {
        let mut input = fixture();
        let after = &mut input["resource_changes"][3]["change"]["after"];
        match case {
            0 => after["destination"] = json!("http_endpoint"),
            1 => after["destination"] = Value::Null,
            2 => after["http_endpoint_configuration"] = json!([{"url": "PRIVATE_ENDPOINT"}]),
            3 => after["future_configuration"] = json!([{}]),
            4 => {
                let block = after["extended_s3_configuration"][0].clone();
                after["extended_s3_configuration"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            }
            5 => after["extended_s3_configuration"] = after["extended_s3_configuration"][0].clone(),
            6 => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"destination": true})
            }
            7 => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"http_endpoint_configuration": true})
            }
            8 => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"extended_s3_configuration": true})
            }
            9 => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"extended_s3_configuration": [{}, {}]})
            }
            10 => input["resource_changes"][3]["change"]["after_unknown"] = json!(true),
            _ => {
                let mut block = input["configuration"]["root_module"]["resources"][3]["expressions"]["extended_s3_configuration"][0].clone();
                block["bucket_arn"] = json!({"references": ["aws_s3_bucket.backup.arn"]});
                block["s3_backup_configuration"][0]["bucket_arn"] =
                    json!({"references": ["aws_s3_bucket.delivery.arn"]});
                input["configuration"]["root_module"]["resources"][3]["expressions"]["extended_s3_configuration"].as_array_mut().unwrap().push(block);
            }
        }
        assert!(!verified(&input, DELIVERY), "case {case}");
        assert!(!verified(&input, BACKUP), "case {case}");
    }
    for case in 0..4 {
        let mut input = fixture();
        match case {
            0 => {
                input["resource_changes"][3]["change"]["after"]["msk_source_configuration"] =
                    json!([{}])
            }
            1 => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"msk_source_configuration": true})
            }
            2 => {
                input["resource_changes"][3]["change"]["after"]["kinesis_source_configuration"] =
                    json!([])
            }
            _ => {
                input["resource_changes"][3]["change"]["after_unknown"] =
                    json!({"kinesis_source_configuration": true})
            }
        }
        assert!(!verified(&input, SOURCE));
        assert!(verified(&input, DELIVERY));
    }
}

#[test]
fn firehose_requires_typed_explicit_current_endpoints_and_preserves_fallback() {
    for (attribute, endpoint, block, field) in [
        (
            SOURCE,
            0,
            "kinesis_source_configuration",
            "kinesis_stream_arn",
        ),
        (DELIVERY, 1, "extended_s3_configuration", "bucket_arn"),
    ] {
        for case in 0..13 {
            let mut input = fixture();
            match case {
                0 => {
                    input["resource_changes"][endpoint]["provider_name"] =
                        json!("example.invalid/foreign/aws")
                }
                1 => input["resource_changes"][endpoint]["type"] = json!("aws_sns_topic"),
                2 => {
                    input["resource_changes"][endpoint]["change"]["after_unknown"] =
                        json!({"arn": true})
                }
                3 => {
                    input["resource_changes"][endpoint]["change"]["after"]["arn"] =
                        json!("different")
                }
                4 => input["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"]),
                5 => input["resource_changes"][endpoint]["deposed"] = json!("old"),
                6 => input["resource_changes"][endpoint]["mode"] = json!("data"),
                7 => input["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
                8 => input["resource_changes"][3]["mode"] = json!("data"),
                9 => {
                    input["configuration"]["root_module"]["resources"][3]["expressions"][block][0]
                        [field] = json!({"constant_value": input["resource_changes"][endpoint]["change"]["after"]["arn"]})
                }
                10 => {
                    input["configuration"]["root_module"]["resources"][3]["expressions"][block][0]
                        [field] = json!({"references": [format!("{}.name", input["resource_changes"][endpoint]["address"].as_str().unwrap())]})
                }
                11 => input["configuration"]["root_module"]["resources"][3]["expressions"][block]
                    [0][field]["references"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("var.unresolved")),
                _ => {
                    input["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .remove(endpoint);
                }
            }
            assert!(!verified(&input, attribute), "{attribute} case {case}");
        }
    }
    for dynamic in [false, true] {
        let mut input = fixture();
        let mut other = input["resource_changes"][0].clone();
        other["address"] = json!("aws_kinesis_stream.input[1]");
        other["index"] = json!(1);
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(other);
        input["configuration"]["root_module"]["resources"][3]["expressions"]["kinesis_source_configuration"]
            [0]["kinesis_stream_arn"] = json!({"references": [if dynamic {"aws_kinesis_stream.input[count.index].arn"} else {"aws_kinesis_stream.input.arn"}]});
        assert!(!verified(&input, SOURCE));
    }
}

#[test]
fn firehose_retains_lifecycle_modules_state_identity_and_private_values() {
    let mut input = fixture();
    input["resource_changes"][3]["previous_address"] =
        json!("aws_kinesis_firehose_delivery_stream.old");
    input["resource_changes"][3]["change"]["importing"] = json!({"id": "PRIVATE_IMPORT"});
    input["resource_changes"][3]["change"]["actions"] = json!(["delete", "create"]);
    for field in [
        "url",
        "credentials",
        "record_payload",
        "processing_configuration",
    ] {
        input["resource_changes"][3]["change"]["after"][field] = json!("PRIVATE_PAYLOAD_<script>");
    }
    let mut old = input["resource_changes"][3].clone();
    old["deposed"] = json!("previous");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    input["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&input.to_string()).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let result = semantic::transform(&parsed);
    assert_eq!(result.coverage.selected.resources, 6);
    let current = result
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".archive") && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(current.action, Action::Replace);
    assert!(current.metadata.import.is_some());
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_kinesis_firehose_delivery_stream.old")
    );
    assert!(
        result
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let text = crate::generate(
            &input.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!text.contains("PRIVATE_"));
        assert!(!text.contains("arn:aws:"));
    }
    let mut modular = fixture();
    let resources = modular["configuration"]["root_module"]["resources"].take();
    modular["configuration"]["root_module"]["module_calls"] =
        json!({"ingest": {"module": {"resources": resources}}});
    for node in modular["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!(
            "module.ingest.{}",
            node["address"].as_str().unwrap()
        ));
        node["module_address"] = json!("module.ingest");
    }
    let output = crate::generate(
        &modular.to_string(),
        &crate::Options {
            modules: vec!["module.ingest".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("5 resources"));
    assert!(output.contains("Kinesis source"));
    assert!(output.contains("S3 backup destination"));
    let mut partial = fixture();
    partial["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let plans = plan::parse_inputs(
        [("one", fixture()), ("two", partial)]
            .into_iter()
            .map(|(name, value)| PlanInput {
                state_id: StateId::new(name).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, plan) in &plans.states {
        let graph = semantic::transform(plan);
        let node = graph
            .nodes
            .iter()
            .find(|n| n.address.ends_with(".archive"))
            .unwrap();
        let source = node
            .references
            .iter()
            .find(|r| r.label == "Kinesis source")
            .unwrap();
        assert_eq!(source.complete, state.as_str() == "one");
        assert!(
            node.references
                .iter()
                .flat_map(|r| &r.targets)
                .all(|t| t.state.is_none())
        );
    }
}
