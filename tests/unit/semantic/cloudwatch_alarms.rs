use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/cloudwatch-alarms-plan.json")).unwrap()
}

fn verified(v: &Value, field: &str, target: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    graph
        .nodes
        .iter()
        .find(|n| n.address == target)
        .unwrap()
        .references
        .iter()
        .any(|r| r.attribute == field && r.complete && r.label.starts_with("Configured"))
}

#[test]
fn alarm_references_require_complete_explicit_current_typed_values() {
    for (target, source, field, output) in [
        (3, 0, "alarm_actions", "arn"),
        (3, 0, "ok_actions", "arn"),
        (3, 0, "insufficient_data_actions", "arn"),
        (4, 3, "actions_suppressor.alarm", "alarm_name"),
        (5, 3, "mute_targets.alarm_names", "alarm_name"),
    ] {
        for case in [
            "valid",
            "unknown",
            "target_unknown",
            "sensitive",
            "mismatch",
            "literal",
            "missing",
            "wrong_type",
            "data",
            "provider",
            "deleted",
            "deposed",
            "region",
            "account",
            "wrong_output",
        ] {
            let mut v = fixture();
            let address = v["resource_changes"][target]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            let top = field.split('.').next().unwrap();
            match case {
                "unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "target_unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({top:true})
                }
                "sensitive" => {
                    v["resource_changes"][source]["change"]["after_sensitive"] =
                        json!({output:true})
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("other")
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"] =
                        json!({})
                }
                "missing" => {
                    v["resource_changes"].as_array_mut().unwrap().remove(source);
                }
                "wrong_type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.invalid/custom/aws")
                }
                "deleted" => {
                    v["resource_changes"][source]["change"]["actions"] = json!(["delete"]);
                    v["resource_changes"][source]["change"]["before"] =
                        v["resource_changes"][source]["change"]["after"].take();
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "region" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                "account" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":"us-east-1"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                "wrong_output" => {
                    let expressions =
                        &mut v["configuration"]["root_module"]["resources"][target]["expressions"];
                    let text = expressions
                        .to_string()
                        .replace(&format!(".{output}\""), ".id\"");
                    *expressions = serde_json::from_str(&text).unwrap();
                }
                _ => {}
            }
            assert_eq!(
                verified(&v, field, &address),
                case == "valid",
                "{field} {case}"
            );
        }
    }
}

#[test]
fn collections_and_suppressor_shapes_are_not_partial_evidence() {
    for case in [
        "extra",
        "duplicate",
        "null",
        "empty",
        "scalar",
        "partial_mask",
        "wrong_arn",
        "ambiguous",
    ] {
        let mut v = fixture();
        let field = &mut v["resource_changes"][3]["change"]["after"]["alarm_actions"];
        match case {
            "extra" => field.as_array_mut().unwrap().push(json!("literal")),
            "duplicate" => {let first = field[0].clone(); field.as_array_mut().unwrap().push(first);},
            "null" => field[1] = Value::Null,
            "empty" => *field = json!([]),
            "scalar" => *field = field[0].clone(),
            "partial_mask" => v["resource_changes"][3]["change"]["after_unknown"] = json!({"alarm_actions":[false,true,false]}),
            "wrong_arn" => {let id = json!("arn:aws:s3:us-east-1:123456789012:bucket"); field[0] = id.clone(); v["resource_changes"][0]["change"]["after"]["arn"] = id;},
            "ambiguous" => v["configuration"]["root_module"]["resources"][3]["expressions"]["alarm_actions"]["references"].as_array_mut().unwrap().push(json!("aws_sns_topic.missing.arn")),
            _ => unreachable!(),
        }
        assert!(
            !verified(&v, "alarm_actions", "aws_cloudwatch_metric_alarm.metric"),
            "{case}"
        );
    }
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"]["actions_suppressor"][0]["alarm"] =
        v["resource_changes"][3]["change"]["after"]["arn"].clone();
    v["configuration"]["root_module"]["resources"][4]["expressions"]["actions_suppressor"][0]["alarm"] =
        json!({"references":["aws_cloudwatch_metric_alarm.metric.arn"]});
    assert!(verified(
        &v,
        "actions_suppressor.alarm",
        "aws_cloudwatch_composite_alarm.composite"
    ));
    let block = v["resource_changes"][4]["change"]["after"]["actions_suppressor"][0].clone();
    v["resource_changes"][4]["change"]["after"]["actions_suppressor"] =
        json!([block.clone(), block]);
    assert!(!verified(
        &v,
        "actions_suppressor.alarm",
        "aws_cloudwatch_composite_alarm.composite"
    ));
}

#[test]
fn configured_links_preserve_independent_actions_selection_and_named_states() {
    let input = fixture().to_string();
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: input.clone(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        let graph = semantic::transform(raw);
        assert_eq!(graph.coverage.selected.resources, 6);
        assert!(
            graph
                .nodes
                .iter()
                .all(|n| n.action == crate::model::Action::Create)
        );
        for focus in [
            None,
            Some("aws_cloudwatch_alarm_mute_rule.maintenance".into()),
        ] {
            let graph = crate::generation::project(
                raw,
                &crate::view::Options {
                    depth: focus.as_ref().map(|_| 0),
                    focus,
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
    }
}

#[test]
fn action_types_and_nested_collections_remain_bounded() {
    for (endpoint, arn, metric_only) in [
        (
            "aws_lambda_alias",
            "arn:aws:lambda:us-east-1:123456789012:function:notify:live",
            false,
        ),
        (
            "aws_appautoscaling_policy",
            "arn:aws:autoscaling:us-east-1:123456789012:scalingPolicy:synthetic:resource/ecs/service/test:policyName/scale",
            true,
        ),
    ] {
        for target in [3, 4] {
            let mut v = fixture();
            let address = format!("{endpoint}.endpoint");
            v["resource_changes"][0]["address"] = json!(address);
            v["resource_changes"][0]["type"] = json!(endpoint);
            v["resource_changes"][0]["change"]["after"]["arn"] = json!(arn);
            v["configuration"]["root_module"]["resources"][0]["address"] = json!(address);
            v["configuration"]["root_module"]["resources"][0]["type"] = json!(endpoint);
            v["configuration"]["root_module"]["resources"][target]["expressions"]["alarm_actions"] =
                json!({"references":[format!("{address}.arn")]});
            v["resource_changes"][target]["change"]["after"]["alarm_actions"] = json!([arn]);
            assert_eq!(
                verified(
                    &v,
                    "alarm_actions",
                    v["resource_changes"][target]["address"].as_str().unwrap()
                ),
                !metric_only || target == 3
            );
        }
    }
    for case in [
        "unknown_member",
        "extra_name",
        "duplicate_name",
        "multiple_blocks",
        "consumer_data",
        "consumer_deleted",
    ] {
        let mut v = fixture();
        match case {
            "unknown_member" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"mute_targets":[{"alarm_names":[false,true]}]})
            }
            "extra_name" => {
                v["resource_changes"][5]["change"]["after"]["mute_targets"][0]["alarm_names"] =
                    json!(["metric", "composite", "literal"])
            }
            "duplicate_name" => {
                v["resource_changes"][4]["change"]["after"]["alarm_name"] = json!("metric");
                v["resource_changes"][5]["change"]["after"]["mute_targets"][0]["alarm_names"] =
                    json!(["metric"]);
            }
            "multiple_blocks" => {
                let block = v["resource_changes"][5]["change"]["after"]["mute_targets"][0].clone();
                v["resource_changes"][5]["change"]["after"]["mute_targets"] =
                    json!([block.clone(), block]);
            }
            "consumer_data" => v["resource_changes"][5]["mode"] = json!("data"),
            "consumer_deleted" => {
                v["resource_changes"][5]["change"]["actions"] = json!(["delete"]);
                v["resource_changes"][5]["change"]["before"] =
                    v["resource_changes"][5]["change"]["after"].take();
            }
            _ => unreachable!(),
        }
        assert!(
            !verified(
                &v,
                "mute_targets.alarm_names",
                "aws_cloudwatch_alarm_mute_rule.maintenance"
            ),
            "{case}"
        );
    }
}
