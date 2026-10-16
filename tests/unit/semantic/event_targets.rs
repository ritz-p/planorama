use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/eventbridge-targets-plan.json")).unwrap()
}

fn graph(input: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&input.to_string()).unwrap())
}

fn targets(graph: &Graph) -> Vec<&Edge> {
    graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::EventTarget)
        .collect()
}

fn jobs_edge(graph: &Graph) -> Option<&Edge> {
    targets(graph)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address == "aws_cloudwatch_event_target.jobs")
}

#[test]
fn event_targets_preserve_direction_provenance_scope_and_order() {
    let mut input = fixture();
    let result = graph(&input);
    assert_eq!(result.nodes.len(), 12);
    assert_eq!(targets(&result).len(), 3);
    for edge in targets(&result) {
        assert_eq!(
            result.nodes[edge.from].resource_type,
            "aws_cloudwatch_event_rule"
        );
        assert_eq!(edge.directionality(), Directionality::Directed);
        let change = edge.change.as_ref().unwrap();
        let source = result
            .nodes
            .iter()
            .find(|n| n.address == change.address)
            .unwrap();
        assert_eq!(source.placement, Placement::Logical);
        assert_eq!(source.action, change.action);
        let relation = result.relationships.iter().find(|r| r.kind == EdgeKind::EventTarget && r.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Resource { change: c, .. } if c == change))).unwrap();
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
    let archive = targets(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address.ends_with(".archive"))
        .unwrap();
    assert_ne!(
        result.nodes[archive.from].entity.scope,
        result.nodes[archive.to].entity.scope
    );
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(result, graph(&input));
}

#[test]
fn event_targets_reject_incomplete_mismatched_and_literal_evidence() {
    for (attribute, expression, value, unknown) in [
        (
            "rule",
            json!({"constant_value":"jobs"}),
            json!("jobs"),
            false,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_rule.jobs.name", "var.missing"]}),
            json!("jobs"),
            false,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_rule.jobs[count.index].name"]}),
            json!("jobs"),
            false,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_rule.jobs.name", "aws_cloudwatch_event_rule.default.name"]}),
            json!("jobs"),
            false,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_rule.jobs.name"]}),
            json!("other"),
            false,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_rule.jobs.name"]}),
            Value::Null,
            true,
        ),
        (
            "rule",
            json!({"references":["aws_cloudwatch_event_bus.events.name"]}),
            json!("events"),
            false,
        ),
        (
            "arn",
            json!({"constant_value":"queue-proof"}),
            json!("queue-proof"),
            false,
        ),
        (
            "arn",
            json!({"references":["aws_sqs_queue.jobs.arn", "var.missing"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "arn",
            json!({"references":["aws_sqs_queue.jobs[count.index].arn"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "arn",
            json!({"references":["aws_sqs_queue.jobs.arn", "aws_lambda_function.worker.arn"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "arn",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            json!("different"),
            false,
        ),
        (
            "arn",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            Value::Null,
            true,
        ),
        (
            "event_bus_name",
            json!({"constant_value":"events"}),
            json!("events"),
            false,
        ),
        (
            "event_bus_name",
            json!({"references":["aws_cloudwatch_event_bus.events.name"]}),
            json!("other"),
            false,
        ),
        (
            "event_bus_name",
            json!({"references":["aws_cloudwatch_event_bus.events.name"]}),
            Value::Null,
            true,
        ),
        (
            "event_bus_name",
            json!({"references":["data.aws_cloudwatch_event_bus.archive.name"]}),
            json!("archive"),
            false,
        ),
    ] {
        let mut input = fixture();
        input["configuration"]["root_module"]["resources"][7]["expressions"][attribute] =
            expression;
        input["resource_changes"][7]["change"]["after"][attribute] = value;
        input["resource_changes"][7]["change"]["after_unknown"][attribute] = json!(unknown);
        assert!(jobs_edge(&graph(&input)).is_none(), "{attribute}: {input}");
    }
    for case in [
        "missing",
        "foreign-rule",
        "foreign-target",
        "foreign-bus",
        "foreign-endpoint",
        "data",
        "deposed",
        "rule-region",
        "bus-region",
        "target-region",
        "rule-bus-literal",
        "rule-bus-mismatch",
        "missing-bus",
        "unknown-name",
    ] {
        let mut input = fixture();
        match case {
            "missing" => {
                input["resource_changes"].as_array_mut().unwrap().remove(4);
            }
            "foreign-rule" => {
                input["resource_changes"][1]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "foreign-target" => {
                input["resource_changes"][7]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "foreign-bus" => {
                input["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "foreign-endpoint" => {
                input["resource_changes"][4]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "data" => input["resource_changes"][7]["mode"] = json!("data"),
            "deposed" => input["resource_changes"][7]["deposed"] = json!("old-key"),
            "rule-region" => {
                input["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                    json!("aws.archive")
            }
            "bus-region" => {
                input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    json!("aws.archive")
            }
            "target-region" => {
                input["configuration"]["root_module"]["resources"][7]["provider_config_key"] =
                    json!("aws.archive")
            }
            "rule-bus-literal" => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["event_bus_name"] =
                    json!({"constant_value":"events"})
            }
            "rule-bus-mismatch" => {
                input["resource_changes"][1]["change"]["after"]["event_bus_name"] = json!("other")
            }
            "missing-bus" => {
                input["configuration"]["root_module"]["resources"][7]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("event_bus_name");
            }
            _ => input["resource_changes"][1]["change"]["after_unknown"]["name"] = json!(true),
        }
        assert!(jobs_edge(&graph(&input)).is_none(), "{case}");
    }
}

#[test]
fn event_targets_handle_optional_default_bus_without_creating_bus_identity() {
    let mut input = fixture();
    for index in [1, 7] {
        input["configuration"]["root_module"]["resources"][index]["expressions"]
            .as_object_mut()
            .unwrap()
            .remove("event_bus_name");
        input["resource_changes"][index]["change"]["after"]["event_bus_name"] = json!("default");
    }
    let result = graph(&input);
    assert!(jobs_edge(&result).is_some());
    assert_eq!(result.nodes.len(), 12);
    for value in [Value::Null, json!("events")] {
        input["resource_changes"][7]["change"]["after"]["event_bus_name"] = value;
        assert!(jobs_edge(&graph(&input)).is_none());
    }
}

#[test]
fn event_targets_verify_accounts_without_borrowing_endpoint_scope() {
    let mut input = fixture();
    for (key, account) in [("aws", "123456789012"), ("aws.archive", "210987654321")] {
        input["configuration"]["provider_config"][key]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/example")}}]);
    }
    let parse = |input: &Value| {
        semantic::transform(
            &crate::plan::evidence::tests::parse_fixture(&input.to_string()).unwrap(),
        )
    };
    let result = parse(&input);
    assert_eq!(targets(&result).len(), 3);
    let edge = targets(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address.ends_with(".archive"))
        .unwrap();
    assert_ne!(
        result.nodes[edge.from]
            .entity
            .scope
            .as_ref()
            .unwrap()
            .account,
        result.nodes[edge.to].entity.scope.as_ref().unwrap().account
    );
    input["configuration"]["provider_config"]["aws.archive"]["expressions"]["region"] =
        json!({"constant_value":"ap-northeast-1"});
    input["configuration"]["root_module"]["resources"][7]["provider_config_key"] =
        json!("aws.archive");
    assert!(jobs_edge(&parse(&input)).is_none());
    assert!(jobs_edge(&graph(&input)).is_none());
}

#[test]
fn event_targets_require_current_endpoints_and_support_replacement_actions() {
    for index in [0, 1, 4] {
        let mut input = fixture();
        input["resource_changes"][index]["deposed"] = json!("old-key");
        assert!(jobs_edge(&graph(&input)).is_none());
    }
    let mut input = fixture();
    input["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    assert_eq!(
        jobs_edge(&graph(&input))
            .unwrap()
            .change
            .as_ref()
            .unwrap()
            .action,
        Action::Replace
    );
    input["configuration"]["root_module"]["resources"][7]["expressions"]["event_bus_name"] =
        json!({"references":["aws_cloudwatch_event_bus.events[count.index].name"]});
    assert!(jobs_edge(&graph(&input)).is_none());
}

#[test]
fn event_targets_preserve_parallel_lifecycle_and_deleted_before_evidence() {
    let mut input = fixture();
    input["resource_changes"][7]["previous_address"] = json!("aws_cloudwatch_event_target.old");
    input["resource_changes"][7]["change"]["importing"] = json!({"id":"private-import"});
    let mut duplicate = input["resource_changes"][7].clone();
    duplicate["address"] = json!("aws_cloudwatch_event_target.second");
    let mut config = input["configuration"]["root_module"]["resources"][7].clone();
    config["address"] = duplicate["address"].clone();
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    let mut deposed = input["resource_changes"][7].clone();
    deposed["deposed"] = json!("old-key");
    deposed["change"]["actions"] = json!(["delete"]);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    let result = graph(&input);
    assert_eq!(targets(&result).len(), 4);
    assert_eq!(result.nodes.len(), 14);
    let change = jobs_edge(&result).unwrap().change.as_ref().unwrap();
    assert_eq!(
        change.previous_address.as_deref(),
        Some("aws_cloudwatch_event_target.old")
    );
    assert!(change.metadata.import.is_some());
    let mut input = fixture();
    for resource in input["resource_changes"].as_array_mut().unwrap() {
        resource["change"]["before"] = resource["change"]["after"].take();
        resource["change"]["actions"] = json!(["delete"]);
    }
    assert_eq!(targets(&graph(&input)).len(), 3);
    input["resource_changes"][4]["change"]["before"]["arn"] = json!("mismatch");
    assert!(jobs_edge(&graph(&input)).is_none());
}
