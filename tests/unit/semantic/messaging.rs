use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/sns-subscriptions-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn subscriptions(graph: &Graph) -> Vec<&Edge> {
    graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Subscription)
        .collect()
}

#[test]
fn deleted_subscriptions_require_matching_before_values_and_explicit_references() {
    let mut input = fixture();
    for change in input["resource_changes"].as_array_mut().unwrap() {
        change["change"]["before"] = change["change"]["after"].take();
        change["change"]["actions"] = json!(["delete"]);
    }
    let result = graph(&input);
    assert_eq!(subscriptions(&result).len(), 3);
    assert!(
        subscriptions(&result)
            .iter()
            .all(|e| e.change.as_ref().unwrap().action == Action::Delete)
    );
    for case in [
        "missing-before",
        "mismatch",
        "missing-reference",
        "new-target",
    ] {
        let mut invalid = input.clone();
        match case {
            "missing-before" => invalid["resource_changes"][1]["change"]["before"] = Value::Null,
            "mismatch" => {
                invalid["resource_changes"][4]["change"]["before"]["endpoint"] = json!("different")
            }
            "missing-reference" => {
                invalid["configuration"]["root_module"]["resources"][4]["expressions"]["endpoint"] =
                    json!({"constant_value":"queue-proof"})
            }
            _ => {
                invalid["resource_changes"][1]["change"]["after"] =
                    invalid["resource_changes"][1]["change"]["before"].take();
                invalid["resource_changes"][1]["change"]["actions"] = json!(["create"]);
            }
        }
        assert!(
            !subscriptions(&graph(&invalid))
                .iter()
                .any(|e| e.change.as_ref().unwrap().address == "aws_sns_topic_subscription.jobs"),
            "{case}"
        );
    }
}

#[test]
fn subscriptions_preserve_direction_identity_actions_and_source_scope() {
    let input = fixture();
    let result = graph(&input);
    assert_eq!(result.nodes.len(), 8);
    assert_eq!(subscriptions(&result).len(), 3);
    for edge in subscriptions(&result) {
        assert_eq!(result.nodes[edge.from].resource_type, "aws_sns_topic");
        assert_eq!(edge.directionality(), Directionality::Directed);
        let change = edge.change.as_ref().unwrap();
        let source = result
            .nodes
            .iter()
            .find(|n| n.address == change.address)
            .unwrap();
        assert_eq!(source.placement, Placement::Logical);
        assert_eq!(source.action, change.action);
        let relation = result.relationships.iter().find(|r| r.kind == EdgeKind::Subscription && r.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Resource { change: c, .. } if c == change))).unwrap();
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
    let email = result
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".email"))
        .unwrap();
    assert!(
        email
            .references
            .iter()
            .any(|r| r.label == "Subscription endpoint" && !r.complete)
    );
    assert_eq!(result, graph(&input));
    let mut reordered = input;
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(result, graph(&reordered));
}

#[test]
fn unproven_subscriptions_keep_logical_fallback_without_false_edges() {
    for (attribute, expression, value, unknown) in [
        (
            "endpoint",
            json!({"constant_value":"queue-proof"}),
            json!("queue-proof"),
            false,
        ),
        (
            "endpoint",
            json!({"references":["aws_sqs_queue.jobs.arn","var.missing"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "endpoint",
            json!({"references":["aws_sqs_queue.jobs.arn","data.aws_sqs_queue.archive.arn"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "endpoint",
            json!({"references":["aws_sqs_queue.jobs[count.index].arn"]}),
            json!("queue-proof"),
            false,
        ),
        (
            "endpoint",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            json!("different-proof"),
            false,
        ),
        (
            "endpoint",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            Value::Null,
            true,
        ),
        (
            "topic_arn",
            json!({"references":["aws_sqs_queue.jobs.arn"]}),
            json!("queue-proof"),
            false,
        ),
    ] {
        let mut input = fixture();
        input["configuration"]["root_module"]["resources"][4]["expressions"][attribute] =
            expression;
        input["resource_changes"][4]["change"]["after"][attribute] = value;
        input["resource_changes"][4]["change"]["after_unknown"][attribute] = json!(unknown);
        let result = graph(&input);
        assert_eq!(subscriptions(&result).len(), 2, "{attribute}: {input}");
        let node = result
            .nodes
            .iter()
            .find(|n| n.address == "aws_sns_topic_subscription.jobs")
            .unwrap();
        assert!(
            node.references
                .iter()
                .any(|r| r.attribute == attribute && !r.complete)
        );
    }
    for protocol in ["lambda", "https", "email", "unknown"] {
        let mut input = fixture();
        input["resource_changes"][4]["change"]["after"]["protocol"] = json!(protocol);
        assert_eq!(subscriptions(&graph(&input)).len(), 2);
    }
}

#[test]
fn subscription_lifecycle_and_duplicate_endpoints_remain_independent() {
    let mut input = fixture();
    input["resource_changes"][4]["previous_address"] = json!("aws_sns_topic_subscription.old");
    input["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut duplicate = input["resource_changes"][4].clone();
    duplicate["address"] = json!("aws_sns_topic_subscription.second");
    let mut config = input["configuration"]["root_module"]["resources"][4].clone();
    config["address"] = duplicate["address"].clone();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let mut deposed = input["resource_changes"][4].clone();
    deposed["deposed"] = json!("old-key");
    deposed["change"]["actions"] = json!(["delete"]);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    let result = graph(&input);
    assert_eq!(subscriptions(&result).len(), 4);
    assert_eq!(result.nodes.len(), 10);
    let edge = subscriptions(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address == "aws_sns_topic_subscription.jobs")
        .unwrap();
    assert_eq!(
        edge.change.as_ref().unwrap().previous_address.as_deref(),
        Some("aws_sns_topic_subscription.old")
    );
    assert!(edge.change.as_ref().unwrap().metadata.import.is_some());
    assert!(
        result
            .nodes
            .iter()
            .find(|n| n.deposed_key.is_some())
            .unwrap()
            .references
            .iter()
            .all(|r| !r.complete)
    );
}

#[test]
fn subscription_boundaries_never_borrow_identity_or_scope() {
    for case in ["missing", "foreign", "data", "deposed", "protocol-unknown"] {
        let mut input = fixture();
        match case {
            "missing" => {
                input["resource_changes"].as_array_mut().unwrap().remove(1);
            }
            "foreign" => {
                input["resource_changes"][1]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "data" => input["resource_changes"][4]["mode"] = json!("data"),
            "deposed" => input["resource_changes"][1]["deposed"] = json!("old"),
            _ => input["resource_changes"][4]["change"]["after_unknown"]["protocol"] = json!(true),
        }
        let result = graph(&input);
        assert!(
            !subscriptions(&result)
                .iter()
                .any(|e| e.change.as_ref().unwrap().address == "aws_sns_topic_subscription.jobs"),
            "{case}"
        );
    }
    let mut input = fixture();
    for (key, account) in [("aws", "123456789012"), ("aws.archive", "210987654321")] {
        input["configuration"]["provider_config"][key]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/example")}}]);
    }
    let raw = crate::plan::evidence::tests::parse_fixture(&input.to_string()).unwrap();
    let result = semantic::transform(&raw);
    let edge = subscriptions(&result)
        .into_iter()
        .find(|e| e.change.as_ref().unwrap().address == "aws_sns_topic_subscription.archive")
        .unwrap();
    assert_eq!(
        result.nodes[edge.from]
            .entity
            .scope
            .as_ref()
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .as_str(),
        "123456789012"
    );
    assert_eq!(
        result.nodes[edge.to]
            .entity
            .scope
            .as_ref()
            .unwrap()
            .account
            .as_ref()
            .unwrap()
            .as_str(),
        "210987654321"
    );
    let source = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_sns_topic_subscription.archive")
        .unwrap();
    assert_eq!(source.entity.scope, result.nodes[edge.from].entity.scope);
    assert_ne!(source.entity.scope, result.nodes[edge.to].entity.scope);
}
