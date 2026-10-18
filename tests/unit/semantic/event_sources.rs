use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/event-sources-plan.json")).unwrap()
}

fn kinesis_fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/kinesis-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn mappings(graph: &Graph) -> Vec<&Edge> {
    graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::EventSource)
        .collect()
}

#[test]
fn kinesis_stream_instances_scope_and_arn_attributes_are_verified() {
    let indexed = kinesis_fixture()
        .to_string()
        .replace("aws_kinesis_stream.jobs", "aws_kinesis_stream.jobs[0]");
    let mut input: Value = serde_json::from_str(&indexed).unwrap();
    input["resource_changes"][1]["index"] = json!(0);
    input["configuration"]["root_module"]["resources"][1]["address"] =
        json!("aws_kinesis_stream.jobs");
    let result = graph(&input);
    assert_eq!(mappings(&result).len(), 3);
    let stream = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_kinesis_stream.jobs[0]")
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
            .map(|r| r.0.as_str()),
        Some("ap-northeast-1")
    );
    for case in ["attribute", "dynamic", "provider", "unknown", "ambiguous"] {
        let mut invalid = input.clone();
        match case {
            "attribute" => {
                invalid["configuration"]["root_module"]["resources"][4]["expressions"]["event_source_arn"] =
                    json!({"references": ["aws_kinesis_stream.jobs[0].name"]})
            }
            "dynamic" => {
                invalid["configuration"]["root_module"]["resources"][4]["expressions"]["event_source_arn"] =
                    json!({"references": ["aws_kinesis_stream.jobs[count.index].arn"]})
            }
            "provider" => {
                invalid["resource_changes"][1]["provider_name"] =
                    json!("example.invalid/foreign/aws")
            }
            "unknown" => {
                invalid["resource_changes"][1]["change"]["after_unknown"]["arn"] = json!(true)
            }
            _ => {
                let mut other = invalid["resource_changes"][1].clone();
                other["address"] = json!("aws_kinesis_stream.jobs[1]");
                other["index"] = json!(1);
                invalid["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
                invalid["configuration"]["root_module"]["resources"][4]["expressions"]["event_source_arn"] =
                    json!({"references": ["aws_kinesis_stream.jobs.arn"]});
            }
        }
        assert!(
            !mappings(&graph(&invalid)).iter().any(|e| e
                .change
                .as_ref()
                .unwrap()
                .address
                .ends_with(".jobs")),
            "{case}"
        );
        assert!(
            semantic::diagnostics::collect(&plan::parse(&invalid.to_string()).unwrap())
                .iter()
                .any(|d| d.address.ends_with("mapping.jobs")
                    && d.reason == DiagnosticReason::UnprovenEndpointValue)
        );
    }
    for region in [json!("eu-west-1"), Value::Null] {
        let mut overridden = input.clone();
        overridden["resource_changes"][1]["change"]["after"]["region"] = region.clone();
        overridden["resource_changes"][1]["change"]["after_unknown"]["region"] =
            json!(region.is_null());
        overridden["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
            json!({"constant_value": region});
        let result = graph(&overridden);
        let stream = result
            .nodes
            .iter()
            .find(|n| n.address == "aws_kinesis_stream.jobs[0]")
            .unwrap();
        assert_eq!(
            stream
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref().map(|r| r.0.as_str())),
            None
        );
    }
}

#[test]
fn deleted_mappings_verify_before_names_and_arns_without_borrowing_after_values() {
    for fixture in [fixture, kinesis_fixture] {
        deleted_mappings_verify_before_names_and_arns_without_borrowing_after_values_with(fixture);
    }
}

fn deleted_mappings_verify_before_names_and_arns_without_borrowing_after_values_with(
    fixture: fn() -> Value,
) {
    let mut input = fixture();
    for change in input["resource_changes"].as_array_mut().unwrap() {
        change["change"]["before"] = change["change"]["after"].take();
        change["change"]["actions"] = json!(["delete"]);
    }
    for delete_endpoints in [true, false] {
        let mut plan = input.clone();
        if !delete_endpoints {
            for index in 0..4 {
                plan["resource_changes"][index]["change"]["actions"] = json!(["update"]);
                plan["resource_changes"][index]["change"]["after"] =
                    json!({"arn":"new-arn","function_name":"new-name"});
            }
        }
        let result = graph(&plan);
        assert_eq!(mappings(&result).len(), 3);
        assert!(
            mappings(&result)
                .iter()
                .all(|e| e.change.as_ref().unwrap().action == Action::Delete)
        );
    }
    for attribute in ["arn", "function_name"] {
        let mut invalid = input.clone();
        invalid["resource_changes"][3]["change"]["after"] =
            invalid["resource_changes"][3]["change"]["before"].clone();
        invalid["resource_changes"][3]["change"]["before"][attribute] = json!("different");
        let expected = if attribute == "arn" { 1 } else { 2 };
        assert_eq!(mappings(&graph(&invalid)).len(), expected, "{attribute}");
    }
}

#[test]
fn event_sources_keep_direction_parallel_identities_and_provenance() {
    for fixture in [fixture, kinesis_fixture] {
        event_sources_keep_direction_parallel_identities_and_provenance_with(fixture);
    }
}

fn event_sources_keep_direction_parallel_identities_and_provenance_with(fixture: fn() -> Value) {
    let input = fixture();
    let result = graph(&input);
    assert_eq!(result.nodes.len(), 9);
    assert_eq!(result.coverage.input.resources, 9);
    assert_eq!(result.coverage.selected.resources, 9);
    assert_eq!(mappings(&result).len(), 3);
    for edge in mappings(&result) {
        assert!(matches!(
            result.nodes[edge.from].resource_type.as_str(),
            "aws_sqs_queue" | "aws_kinesis_stream"
        ));
        assert_eq!(result.nodes[edge.to].resource_type, "aws_lambda_function");
        assert_eq!(edge.directionality(), Directionality::Directed);
        let change = edge.change.as_ref().unwrap();
        let source = result
            .nodes
            .iter()
            .find(|n| n.address == change.address)
            .unwrap();
        assert_eq!(source.placement, Placement::Logical);
        assert_eq!(source.action, change.action);
        let relation = result.relationships.iter().find(|r| r.kind == EdgeKind::EventSource && r.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Resource { change: c, .. } if c == change))).unwrap();
        let RelationshipProvenance::Resource {
            source: id,
            deployment: Some(deployment),
            ..
        } = &relation.provenance[0]
        else {
            panic!()
        };
        assert_eq!(id, &source.entity.provenance[0]);
        assert_eq!(deployment.scope, source.entity.scope);
    }
    let mut reversed = input;
    reversed["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reversed["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(result, graph(&reversed));
}

#[test]
fn event_sources_require_complete_unique_matching_source_and_function_evidence() {
    for fixture in [fixture, kinesis_fixture] {
        event_sources_require_complete_unique_matching_source_and_function_evidence_with(fixture);
    }
}

fn event_sources_require_complete_unique_matching_source_and_function_evidence_with(
    fixture: fn() -> Value,
) {
    for (attribute, expression, value, unknown) in [
        (
            "event_source_arn",
            json!({"constant_value":"queue-proof"}),
            json!("queue-proof"),
            false,
        ),
        (
            "function_name",
            json!({"constant_value":"worker-proof"}),
            json!("worker-proof"),
            false,
        ),
        (
            "function_name",
            json!({"references":["aws_lambda_function.worker.function_name", "var.missing"]}),
            json!("worker-proof"),
            false,
        ),
        (
            "function_name",
            json!({"references":["aws_lambda_function.worker.function_name"]}),
            json!("different"),
            false,
        ),
        (
            "event_source_arn",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            json!("different"),
            false,
        ),
        (
            "function_name",
            json!({"references":["aws_lambda_function.worker.function_name"]}),
            Value::Null,
            true,
        ),
        (
            "event_source_arn",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            Value::Null,
            true,
        ),
        (
            "event_source_arn",
            json!({"references":["aws_sqs_queue.jobs.arn", "data.aws_sqs_queue.archive.arn"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "function_name",
            json!({"references":["aws_lambda_function.worker[count.index].function_name"]}),
            json!("worker-proof"),
            false,
        ),
        (
            "function_name",
            json!({"references":["aws_sns_topic.events.arn"]}),
            json!("topic-proof"),
            false,
        ),
    ] {
        let mut input = fixture();
        let expression = serde_json::from_str::<Value>(&expression.to_string().replace(
            "aws_sqs_queue",
            input["resource_changes"][1]["type"].as_str().unwrap(),
        ))
        .unwrap();
        let value = if value == json!("queue-proof") {
            input["resource_changes"][1]["change"]["after"]["arn"].clone()
        } else {
            value
        };
        input["configuration"]["root_module"]["resources"][4]["expressions"][attribute] =
            expression;
        input["resource_changes"][4]["change"]["after"][attribute] = value;
        input["resource_changes"][4]["change"]["after_unknown"][attribute] = json!(unknown);
        let result = graph(&input);
        assert_eq!(mappings(&result).len(), 2, "{attribute}: {input}");
        let node = result
            .nodes
            .iter()
            .find(|n| n.address == "aws_lambda_event_source_mapping.jobs")
            .unwrap();
        assert!(
            node.references
                .iter()
                .any(|r| r.attribute == attribute && !r.complete)
        );
    }
    for resource_type in [
        "aws_kinesis_stream_consumer",
        "aws_dynamodb_table",
        "aws_msk_cluster",
        "aws_mq_broker",
        "aws_docdb_cluster",
    ] {
        let mut input = fixture();
        input["resource_changes"][1]["type"] = json!(resource_type);
        assert!(
            !mappings(&graph(&input)).iter().any(|e| e
                .change
                .as_ref()
                .unwrap()
                .address
                .ends_with(".jobs"))
        );
    }
}

#[test]
fn event_source_lifecycle_scopes_modes_and_historical_versions_stay_independent() {
    for fixture in [fixture, kinesis_fixture] {
        event_source_lifecycle_scopes_modes_and_historical_versions_stay_independent_with(fixture);
    }
}

fn event_source_lifecycle_scopes_modes_and_historical_versions_stay_independent_with(
    fixture: fn() -> Value,
) {
    let mut input = fixture();
    let address = format!(
        "aws_lambda_event_source_mapping.jobs[\"{}\"]",
        "long.key.".repeat(18)
    );
    input["resource_changes"][4]["address"] = json!(address);
    input["resource_changes"][4]["previous_address"] = json!("aws_lambda_event_source_mapping.old");
    input["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    input["resource_changes"][4]["change"]["actions"] = json!(["delete", "create"]);
    let mut deposed = input["resource_changes"][4].clone();
    deposed["deposed"] = json!("previous");
    deposed["change"]["actions"] = json!(["delete"]);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    for (key, account) in [("aws", "123456789012"), ("aws.archive", "210987654321")] {
        input["configuration"]["provider_config"][key]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/example")}}]);
    }
    let result = semantic::transform(
        &crate::plan::evidence::tests::parse_fixture(&input.to_string()).unwrap(),
    );
    assert_eq!(mappings(&result).len(), 3);
    let change = mappings(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address == address)
        .unwrap()
        .change
        .as_ref()
        .unwrap();
    assert_eq!(change.action, Action::Replace);
    assert!(change.metadata.import.is_some());
    assert_eq!(
        change.previous_address.as_deref(),
        Some("aws_lambda_event_source_mapping.old")
    );
    let old = result
        .nodes
        .iter()
        .find(|n| n.deposed_key.is_some())
        .unwrap();
    assert_eq!(old.placement, Placement::Logical);
    assert!(old.references.iter().all(|r| !r.complete));
    let archive = mappings(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address.ends_with(".archive"))
        .unwrap();
    assert_ne!(
        result.nodes[archive.from].entity.scope,
        result.nodes[archive.to].entity.scope
    );
    let owner = result
        .nodes
        .iter()
        .find(|n| n.address.ends_with("mapping.archive"))
        .unwrap();
    assert_eq!(owner.entity.scope, result.nodes[archive.to].entity.scope);
    let layout = crate::layout::Layout::new(&result);
    let svg = crate::svg::render(&result, &layout);
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some(address.as_str()))
            .count(),
        2
    );
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_attribute("data-relationship-entry"))
            .count(),
        3
    );
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_attribute("data-relationship-marker"))
            .count(),
        6
    );
    assert!(!svg.contains("PRIVATE_IMPORT"));
    assert!(svg.contains("Moved from:"));
    for (index, edge) in result.edges.iter().enumerate() {
        if edge.kind == EdgeKind::EventSource {
            assert!(layout.paths[index].len() >= 2);
        }
    }
    for case in ["missing", "foreign", "data", "deposed"] {
        let mut input = fixture();
        match case {
            "missing" => {
                input["resource_changes"].as_array_mut().unwrap().remove(3);
            }
            "foreign" => {
                input["resource_changes"][3]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "data" => input["resource_changes"][4]["mode"] = json!("data"),
            _ => input["resource_changes"][3]["deposed"] = json!("previous"),
        }
        assert!(
            !mappings(&graph(&input)).iter().any(|e| e
                .change
                .as_ref()
                .unwrap()
                .address
                .ends_with(".jobs")),
            "{case}"
        );
    }
}
