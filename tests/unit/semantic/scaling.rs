use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn autoscaling_auxiliary_references_require_complete_current_matching_evidence() {
    for (address, field) in [
        ("aws_autoscaling_group_tag.helper", "autoscaling_group_name"),
        ("aws_autoscaling_schedule.helper", "autoscaling_group_name"),
        (
            "aws_autoscaling_lifecycle_hook.helper",
            "autoscaling_group_name",
        ),
        (
            "aws_autoscaling_lifecycle_hook.helper",
            "notification_target_arn",
        ),
        ("aws_autoscaling_lifecycle_hook.helper", "role_arn"),
        ("aws_autoscaling_notification.groups", "group_names"),
        ("aws_autoscaling_notification.groups", "topic_arn"),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "mismatch",
            "partial",
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
            match case {
                "unknown" => resource["change"]["after_unknown"][field] = json!(true),
                "mismatch" => {
                    resource["change"]["after"][field] = if field == "group_names" {
                        json!(["other"])
                    } else {
                        json!("wrong")
                    }
                }
                "partial" if field == "group_names" => resource["change"]["after"][field]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("missing")),
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
                    .remove(field);
            }
            if case == "unknown_region" {
                v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]});
            }
            let raw = plan::parse(&v.to_string()).unwrap();
            let expected = case == "valid" || (case == "partial" && field != "group_names");
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field} {case}"
            );
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.coverage.selected.resources,
                v["resource_changes"].as_array().unwrap().len()
            );
            if case == "valid" {
                assert!(
                    graph
                        .nodes
                        .iter()
                        .find(|n| n.address == address)
                        .unwrap()
                        .member_owner
                        .is_none()
                );
                let selected = crate::view::single(
                    &graph,
                    &crate::view::Options {
                        focus: Some(address.into()),
                        depth: Some(0),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(selected.coverage.selected.resources, 1);
            }
        }
    }
}
