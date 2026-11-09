use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn scaling_tuples_require_exact_formatted_ids_qualifiers_and_complete_instructions() {
    for (address, field) in [
        ("aws_appautoscaling_target.table", "resource_id"),
        ("aws_appautoscaling_target.index", "resource_id"),
        ("aws_appautoscaling_target.database", "resource_id"),
        ("aws_appautoscaling_target.service", "resource_id"),
        ("aws_appautoscaling_target.table", "role_arn"),
        ("aws_appautoscaling_policy.table", "resource_id"),
        (
            "aws_appautoscaling_scheduled_action.table",
            "scalable_dimension",
        ),
        (
            "aws_autoscalingplans_scaling_plan.application",
            "scaling_instruction.resource_id",
        ),
        (
            "aws_autoscalingplans_scaling_plan.application",
            "application_source.cloudformation_stack_arn",
        ),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "mismatch",
            "wrong_namespace",
            "wrong_dimension",
            "deleted",
            "data",
            "deposed",
            "wrong_provider",
            "unknown_region",
        ] {
            let mut v: Value =
                serde_json::from_str(include_str!("../../fixtures/autoscaling-policy-plan.json"))
                    .unwrap();
            let resource = v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            let root = field.split('.').next().unwrap();
            match case {
                "unknown" => resource["change"]["after_unknown"][root] = json!(true),
                "mismatch" => {
                    if root == "scaling_instruction" {
                        resource["change"]["after"][root][0]["resource_id"] =
                            json!("table/suffix-guess");
                    } else if root == "application_source" {
                        resource["change"]["after"][root][0]["cloudformation_stack_arn"] =
                            json!("wrong");
                    } else {
                        resource["change"]["after"][field] = json!("wrong");
                    }
                }
                "wrong_namespace" => {
                    if root == "scaling_instruction" {
                        resource["change"]["after"][root][0]["service_namespace"] =
                            json!("custom-resource");
                    } else {
                        resource["change"]["after"]["service_namespace"] = json!("custom-resource");
                    }
                }
                "wrong_dimension" => {
                    if root == "scaling_instruction" {
                        resource["change"]["after"][root][0]["scalable_dimension"] = json!("wrong");
                    } else {
                        resource["change"]["after"]["scalable_dimension"] = json!("wrong");
                    }
                }
                "deleted" => resource["change"]["actions"] = json!(["delete"]),
                "data" => resource["mode"] = json!("data"),
                "deposed" => resource["deposed"] = json!("old"),
                "wrong_provider" => resource["provider_name"] = json!("example.invalid/custom/aws"),
                _ => {}
            }
            if case == "literal" {
                v["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == address)
                    .unwrap()["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(root);
            }
            if case == "unknown_region" {
                v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]});
            }
            let raw = plan::parse(&v.to_string()).unwrap();
            let expected = case == "valid"
                || (matches!(case, "wrong_namespace" | "wrong_dimension")
                    && matches!(
                        field,
                        "role_arn" | "application_source.cloudformation_stack_arn"
                    ));
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field} {case}"
            );
            assert_eq!(
                semantic::transform(&raw).coverage.selected.resources,
                v["resource_changes"].as_array().unwrap().len()
            );
        }
    }
}

#[test]
fn scaling_formatted_ids_reject_missing_cluster_index_and_partial_instructions() {
    for case in ["cluster", "index", "partial", "duplicate"] {
        let mut v: Value =
            serde_json::from_str(include_str!("../../fixtures/autoscaling-policy-plan.json"))
                .unwrap();
        if case == "cluster" {
            v["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_ecs_service.scaling")
                .unwrap()["expressions"] = json!({});
        }
        if case == "index" {
            v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_dynamodb_table.scaling")
                .unwrap()["change"]["after_unknown"]["global_secondary_index"] = json!(true);
        }
        if matches!(case, "partial" | "duplicate") {
            let blocks = v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_autoscalingplans_scaling_plan.application")
                .unwrap()["change"]["after"]["scaling_instruction"]
                .as_array_mut()
                .unwrap();
            let mut extra = blocks[0].clone();
            if case == "partial" {
                extra["resource_id"] = json!("table/missing");
            }
            blocks.push(extra);
        }
        let raw = plan::parse(&v.to_string()).unwrap();
        assert!(
            !raw.attributes
                .iter()
                .any(|r| r.attribute == "scaling_instruction.resource_id"
                    && r.service_reference_verified),
            "{case}"
        );
    }
}
