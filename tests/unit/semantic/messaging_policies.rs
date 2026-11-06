use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/messaging-policies-plan.json");

#[test]
fn policy_owners_require_unique_explicit_current_typed_outputs() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (target, source, path, label) in [
        (1, 0, "arn", "Policy topic"),
        (2, 0, "arn", "Policy topic"),
        (4, 3, "queue_url", "Policy queue"),
        (5, 3, "queue_url", "Policy queue"),
        (6, 3, "queue_url", "Policy queue"),
    ] {
        let mut pair = fixture.clone();
        pair["resource_changes"] = json!([
            fixture["resource_changes"][source],
            fixture["resource_changes"][target]
        ]);
        pair["configuration"]["root_module"]["resources"] = json!([
            fixture["configuration"]["root_module"]["resources"][source],
            fixture["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
        for case in [
            "sensitive",
            "endpoint-sensitive",
            "wrong-output",
            "region",
            "binding",
            "move",
            "import",
            "replace",
        ] {
            let mut v = pair.clone();
            match case {
                "sensitive" => {
                    v["resource_changes"][1]["change"]["after_sensitive"] = json!({path:true})
                }
                "endpoint-sensitive" => {
                    v["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"arn":true,"url":true})
                }
                "wrong-output" => {
                    let address = v["resource_changes"][0]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][1]["expressions"][path] =
                        json!({"references":[format!("{address}.name")]});
                }
                "region" => {
                    v["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                "binding" => {
                    v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                "move" => {
                    v["resource_changes"][1]["previous_address"] =
                        json!("module.old.aws_policy.main")
                }
                "import" => {
                    v["resource_changes"][1]["change"]["importing"] = json!({"id":"import-policy"})
                }
                _ => {
                    v["resource_changes"][1]["change"]["actions"] = json!(["delete", "create"]);
                    v["resource_changes"][1]["change"]["before"] =
                        v["resource_changes"][1]["change"]["after"].clone();
                }
            }
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            assert_eq!(
                graph.nodes[1]
                    .references
                    .iter()
                    .any(|r| r.label == label && r.complete),
                matches!(case, "move" | "import" | "replace"),
                "{target} {case}"
            );
            assert_eq!(graph.coverage.selected.resources, 2);
        }
    }
}

#[test]
fn policies_remain_independent_and_queue_url_is_not_an_arn_contract() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
    for node in graph.nodes.iter().filter(|n| {
        super::label(
            &n.resource_type,
            if n.resource_type.starts_with("aws_sns") {
                "arn"
            } else {
                "queue_url"
            },
        )
        .is_some()
    }) {
        assert_eq!(node.placement, crate::model::Placement::Logical);
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let result = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!result.content.contains("PRIVATE_"));
        assert!(result.content.contains("Policy topic"));
        assert!(result.content.contains("Policy queue"));
    }
    for target in [4, 5, 6] {
        v["resource_changes"][target]["change"]["after"]["queue_url"] =
            v["resource_changes"][3]["change"]["after"]["arn"].clone();
        v["configuration"]["root_module"]["resources"][target]["expressions"]["queue_url"] =
            json!({"references":["aws_sqs_queue.main.arn"]});
    }
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Policy queue" && r.complete)
    );
    v["resource_changes"][4]["change"]["after"]["queue_url"] =
        json!("https://sqs.us-east-1.amazonaws.com/123456789012/main");
    v["configuration"]["root_module"]["resources"][4]["expressions"]["queue_url"] =
        json!({"references":["aws_sqs_queue.main.id"]});
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        graph.nodes[4]
            .references
            .iter()
            .any(|r| r.label == "Policy queue" && r.complete)
    );
}
