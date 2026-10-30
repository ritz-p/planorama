use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn qualified_helpers_preserve_named_states_actions_and_filtered_links() {
    let input = include_str!("../../fixtures/lambda-qualified-plan.json");
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: input.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        let graph = semantic::transform(raw);
        assert_eq!(graph.coverage.selected.resources, 7);
        for focus in [None, Some("aws_lambda_invocation.config".into())] {
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
            assert!(!svg.contains("PRIVATE_"));
            let doc = roxmltree::Document::parse(&svg).unwrap();
            for href in doc
                .descendants()
                .filter_map(|n| n.attribute("href"))
                .filter_map(|h| h.strip_prefix('#'))
            {
                assert!(doc.descendants().any(|n| n.attribute("id") == Some(href)));
            }
        }
    }
    for actions in [
        json!(["update"]),
        json!(["delete", "create"]),
        json!(["no-op"]),
    ] {
        let mut v = fixture("aws_lambda_invocation");
        v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["change"]["actions"] = actions;
        assert!(verified(&v, "function_name"));
    }
}

fn fixture(kind: &str) -> Value {
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/lambda-plan.json")).unwrap();
    let address = format!("{kind}.config");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"function_name":"synthetic-function","qualifier":"7","input":"PAYLOAD_NOT_FOR_OUTPUT","result":"RESULT_NOT_FOR_OUTPUT"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_config_key":"aws","expressions":{"function_name":{"references":["aws_lambda_function.app.function_name"]},"qualifier":{"references":["aws_lambda_function.app.version"]}}}));
    v
}

fn verified(v: &Value, field: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    semantic::transform(&raw)
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".config"))
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && !r.dependency
                && matches!(r.label, "Configured Lambda function" | "Function qualifier")
        })
}

#[test]
fn qualified_configuration_requires_exact_parent_version_and_current_provenance() {
    for kind in [
        "aws_lambda_function_scaling_config",
        "aws_lambda_invocation",
        "aws_lambda_provisioned_concurrency_config",
        "aws_lambda_runtime_management_config",
    ] {
        for case in [
            "valid",
            "unknown",
            "mismatch",
            "literal",
            "wrong_parent",
            "deleted",
            "deposed",
            "data",
            "provider",
            "region",
        ] {
            let mut v = fixture(kind);
            let last = v["resource_changes"].as_array().unwrap().len() - 1;
            let config = v["configuration"]["root_module"]["resources"]
                .as_array()
                .unwrap()
                .len()
                - 1;
            match case {
                "unknown" => {
                    v["resource_changes"][last]["change"]["after_unknown"] =
                        json!({"qualifier":true})
                }
                "mismatch" => {
                    v["resource_changes"][last]["change"]["after"]["qualifier"] = json!("8")
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][config]["expressions"]["qualifier"] =
                        json!({"constant_value":"7"})
                }
                "wrong_parent" => {
                    v["configuration"]["root_module"]["resources"][config]["expressions"]["qualifier"] =
                        json!({"references":["aws_lambda_layer_version.shared[0].version"]})
                }
                "deleted" => {
                    v["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
                    v["resource_changes"][0]["change"]["before"] =
                        v["resource_changes"][0]["change"]["after"].take();
                }
                "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
                "data" => v["resource_changes"][0]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws")
                }
                "region" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    v["configuration"]["root_module"]["resources"][config]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => {}
            }
            assert_eq!(
                verified(&v, "function_name"),
                case == "valid",
                "{kind} {case}"
            );
            assert_eq!(verified(&v, "qualifier"), case == "valid", "{kind} {case}");
        }
    }
}

#[test]
fn qualifier_special_values_and_aliases_are_api_specific() {
    for (kind, selected, expected) in [
        (
            "aws_lambda_function_scaling_config",
            "$LATEST.PUBLISHED",
            true,
        ),
        ("aws_lambda_function_scaling_config", "$LATEST", false),
        ("aws_lambda_runtime_management_config", "$LATEST", true),
        (
            "aws_lambda_runtime_management_config",
            "$LATEST.PUBLISHED",
            false,
        ),
        ("aws_lambda_invocation", "$LATEST", true),
        (
            "aws_lambda_provisioned_concurrency_config",
            "$LATEST",
            false,
        ),
    ] {
        let mut v = fixture(kind);
        v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["change"]["after"]["qualifier"] = json!(selected);
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["expressions"]
            .as_object_mut()
            .unwrap()
            .insert("qualifier".into(), json!({"constant_value":selected}));
        assert_eq!(verified(&v, "function_name"), expected, "{kind} {selected}");
    }
    for kind in [
        "aws_lambda_provisioned_concurrency_config",
        "aws_lambda_invocation",
        "aws_lambda_function_scaling_config",
        "aws_lambda_runtime_management_config",
    ] {
        let mut v = fixture(kind);
        v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["change"]["after"]["qualifier"] = json!("live");
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()["expressions"]["qualifier"] =
            json!({"references":["aws_lambda_alias.live.name"]});
        assert_eq!(
            verified(&v, "qualifier"),
            matches!(
                kind,
                "aws_lambda_provisioned_concurrency_config" | "aws_lambda_invocation"
            ),
            "{kind}"
        );
    }
    assert!(verified(
        &fixture("aws_lambda_function_recursion_config"),
        "function_name"
    ));
}

#[test]
fn qualified_alias_parent_and_sensitive_incomplete_evidence_stay_unresolved() {
    for case in [
        "alias_parent_literal",
        "unknown_output",
        "sensitive_output",
        "missing_function",
        "multiple_sources",
        "malformed_mask",
    ] {
        let mut v = fixture("aws_lambda_provisioned_concurrency_config");
        let last = v["resource_changes"].as_array().unwrap().len() - 1;
        let config = v["configuration"]["root_module"]["resources"]
            .as_array()
            .unwrap()
            .len()
            - 1;
        v["resource_changes"][last]["change"]["after"]["qualifier"] = json!("live");
        v["configuration"]["root_module"]["resources"][config]["expressions"]["qualifier"] =
            json!({"references":["aws_lambda_alias.live.name"]});
        match case {
            "alias_parent_literal" => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["function_name"] =
                    json!({"constant_value":"synthetic-function"})
            }
            "unknown_output" => {
                v["resource_changes"][3]["change"]["after_unknown"] = json!({"name":true})
            }
            "sensitive_output" => {
                v["resource_changes"][3]["change"]["after_sensitive"] = json!({"name":true})
            }
            "missing_function" => {
                v["configuration"]["root_module"]["resources"][config]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("function_name");
            }
            "multiple_sources" => {
                v["configuration"]["root_module"]["resources"][config]["expressions"]["qualifier"] = json!({"references":["aws_lambda_alias.live.name","aws_lambda_alias.canary.name"]})
            }
            _ => {
                v["resource_changes"][last]["change"]["after_unknown"] =
                    json!({"qualifier":{"invalid":true}})
            }
        }
        assert!(!verified(&v, "qualifier"), "{case}");
    }
}

#[test]
fn alias_function_name_indirection_rechecks_masked_parent_outputs() {
    for sensitive in [false, true] {
        let mut v = fixture("aws_lambda_invocation");
        let last = v["resource_changes"].as_array().unwrap().len() - 1;
        let config = v["configuration"]["root_module"]["resources"]
            .as_array()
            .unwrap()
            .len()
            - 1;
        let arn = v["resource_changes"][0]["change"]["after"]["arn"].clone();
        v["resource_changes"][3]["change"]["after"]["function_name"] = arn.clone();
        v["resource_changes"][last]["change"]["after"]["function_name"] = arn;
        v["resource_changes"][last]["change"]["after"]["qualifier"] = Value::Null;
        v["configuration"]["root_module"]["resources"][2]["expressions"]["function_name"] =
            json!({"references":["aws_lambda_function.app.arn"]});
        v["configuration"]["root_module"]["resources"][config]["expressions"] =
            json!({"function_name":{"references":["aws_lambda_alias.live.function_name"]}});
        v["resource_changes"][0]["change"]["after_sensitive"] = json!({"arn":sensitive});
        assert_eq!(verified(&v, "function_name"), !sensitive);
    }
}
