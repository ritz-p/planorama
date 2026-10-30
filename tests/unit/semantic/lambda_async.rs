use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn async_named_states_filtering_and_account_boundaries_preserve_helpers() {
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: fixture().to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        assert_eq!(semantic::transform(raw).coverage.selected.resources, 5);
        let graph = crate::generation::project(
            raw,
            &crate::view::Options {
                focus: Some("aws_lambda_function_event_invoke_config.config".into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
        let doc = roxmltree::Document::parse(&svg).unwrap();
        for href in doc
            .descendants()
            .filter_map(|n| n.attribute("href"))
            .filter_map(|s| s.strip_prefix('#'))
        {
            assert!(doc.descendants().any(|n| n.attribute("id") == Some(href)));
        }
    }
    let mut raw = plan::parse(&fixture().to_string()).unwrap();
    for n in &mut raw.nodes {
        n.provider_configuration.as_mut().unwrap().account = Some(
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_sqs_queue" {
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
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Async failure destination" && r.complete)
    );
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/lambda-async-plan.json")).unwrap()
}

#[test]
fn async_qualifiers_and_lambda_destination_qualification_keep_parent_identity() {
    for selector in [
        json!(null),
        json!("$LATEST"),
        json!("7"),
        json!("$LATEST.PUBLISHED"),
    ] {
        let mut v = fixture();
        v["resource_changes"][4]["change"]["after"]["qualifier"] = selector.clone();
        let expressions = &mut v["configuration"]["root_module"]["resources"][4]["expressions"];
        expressions.as_object_mut().unwrap().remove("qualifier");
        if selector == json!("7") {
            expressions["qualifier"] = json!({"references":["aws_lambda_function.app.version"]});
        }
        assert_eq!(
            complete(&v, "on_failure"),
            selector != json!("$LATEST.PUBLISHED")
        );
    }
    for alias in [false, true] {
        for valid in [false, true] {
            let mut v = fixture();
            let arn = "arn:aws:lambda:us-east-1:123456789012:function:synthetic-function";
            let qualified = format!("{arn}:{}", if alias { "live" } else { "7" });
            let (index, field, address) = if alias {
                (1, "arn", "aws_lambda_alias.live")
            } else {
                (0, "qualified_arn", "aws_lambda_function.app")
            };
            v["resource_changes"][index]["change"]["after"][field] = json!(qualified);
            if !valid {
                v["resource_changes"][index]["change"]["after"]
                    [if alias { "name" } else { "version" }] = json!("other");
            }
            v["resource_changes"][4]["change"]["after"]["destination_config"][0]["on_failure"][0]
                ["destination"] = json!(qualified);
            v["configuration"]["root_module"]["resources"][4]["expressions"]["destination_config"]
                [0]["on_failure"][0]["destination"] =
                json!({"references":[format!("{address}.{field}")]});
            assert_eq!(complete(&v, "on_failure"), valid, "alias={alias}");
        }
    }
}
fn complete(v: &Value, branch: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_lambda_function_event_invoke_config")
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.complete
                && r.attribute == format!("destination_config.{branch}.destination")
                && matches!(
                    r.label,
                    "Async success destination" | "Async failure destination"
                )
        })
}

#[test]
fn async_destinations_require_complete_typed_values_and_qualified_parent() {
    for case in [
        "valid",
        "fifo",
        "unknown_fifo",
        "unknown_destination",
        "partial_block",
        "unknown_block",
        "literal",
        "mismatch",
        "wrong_parent",
        "deleted",
        "data",
        "sensitive",
        "wrong_type",
    ] {
        let mut v = fixture();
        match case {
            "fifo" => v["resource_changes"][2]["change"]["after"]["fifo_queue"] = json!(true),
            "unknown_fifo" => {
                v["resource_changes"][2]["change"]["after_unknown"] = json!({"fifo_queue":true})
            }
            "unknown_destination" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"destination_config":[{"on_failure":[{"destination":true}]}]})
            }
            "partial_block" => {
                let block =
                    v["resource_changes"][4]["change"]["after"]["destination_config"][0].clone();
                v["resource_changes"][4]["change"]["after"]["destination_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            }
            "unknown_block" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"destination_config":true})
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["destination_config"]
                    [0]["on_failure"][0]["destination"] =
                    json!({"constant_value":"arn:aws:sqs:us-east-1:123456789012:failure"})
            }
            "mismatch" => {
                v["resource_changes"][4]["change"]["after"]["destination_config"][0]["on_failure"]
                    [0]["destination"] = json!("other")
            }
            "wrong_parent" => {
                v["resource_changes"][1]["change"]["after"]["function_name"] = json!("other")
            }
            "deleted" => {
                v["resource_changes"][2]["change"]["actions"] = json!(["delete"]);
                v["resource_changes"][2]["change"]["before"] =
                    v["resource_changes"][2]["change"]["after"].take();
            }
            "data" => v["resource_changes"][2]["mode"] = json!("data"),
            "sensitive" => {
                v["resource_changes"][2]["change"]["after_sensitive"] = json!({"arn":true})
            }
            "wrong_type" => v["resource_changes"][2]["type"] = json!("aws_iam_role"),
            _ => {}
        }
        assert_eq!(complete(&v, "on_failure"), case == "valid", "{case}");
        if case == "valid" {
            assert!(complete(&v, "on_success"));
        }
    }
}

#[test]
fn async_destination_types_distinguish_failure_only_s3() {
    for kind in [
        "aws_s3_bucket",
        "aws_cloudwatch_event_bus",
        "aws_lambda_function",
        "aws_sns_topic",
        "aws_sqs_queue",
        "aws_kinesis_stream",
    ] {
        for branch in ["on_success", "on_failure"] {
            let mut v = fixture();
            let address = format!("{kind}.destination");
            v["resource_changes"][2]["type"] = json!(kind);
            v["resource_changes"][2]["address"] = json!(address);
            v["resource_changes"][2]["change"]["after"]["fifo_topic"] = json!(false);
            v["configuration"]["root_module"]["resources"][2]["type"] = json!(kind);
            v["configuration"]["root_module"]["resources"][2]["address"] = json!(address);
            let arn = match kind {
                "aws_s3_bucket" => "arn:aws:s3:::archive",
                "aws_cloudwatch_event_bus" => {
                    "arn:aws:events:us-east-1:123456789012:event-bus/events"
                }
                "aws_lambda_function" => {
                    "arn:aws:lambda:us-east-1:123456789012:function:destination"
                }
                _ => "arn:aws:sqs:us-east-1:123456789012:destination",
            };
            v["resource_changes"][2]["change"]["after"]["arn"] = json!(arn);
            v["resource_changes"][4]["change"]["after"]["destination_config"][0][branch] =
                json!([{"destination":arn}]);
            v["configuration"]["root_module"]["resources"][4]["expressions"]["destination_config"]
                [0][branch] = json!([{"destination":{"references":[format!("{address}.arn")]}}]);
            assert_eq!(
                complete(&v, branch),
                kind != "aws_kinesis_stream" && (kind != "aws_s3_bucket" || branch == "on_failure"),
                "{kind} {branch}"
            );
        }
    }
}
