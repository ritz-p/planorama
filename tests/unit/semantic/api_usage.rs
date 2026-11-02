use crate::{plan, semantic};
use serde_json::{Value, json};
fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/api-usage-plan.json")).unwrap()
}
fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}
#[test]
fn complete_api_stage_pairs_and_key_type_preserve_shared_identity() {
    let v = input();
    for field in [
        "api_stages.api_id",
        "api_stages.stage",
        "key_id",
        "usage_plan_id",
    ] {
        assert!(verified(&v, field), "{field}");
    }
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(
                |n| n.resource_type.starts_with("aws_api_gateway_usage_plan")
                    && n.member_owner.is_none()
            )
            .count(),
        2
    );
    for scenario in 0..10 {
        let mut bad = v.clone();
        match scenario {
            0 => bad["resource_changes"][0]["change"]["after"]["id"] = json!("other"),
            1 => bad["resource_changes"][2]["change"]["after"]["rest_api_id"] = json!("api-two"),
            2 => bad["resource_changes"][2]["change"]["after_unknown"]["stage_name"] = json!(true),
            3 => bad["resource_changes"][4]["change"]["after_unknown"]["api_stages"] = json!(true),
            4 => {
                bad["resource_changes"][4]["change"]["after"]["api_stages"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            5 => bad["resource_changes"][2]["mode"] = json!("data"),
            6 => bad["resource_changes"][2]["deposed"] = json!("old"),
            7 => bad["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            8 => {
                bad["configuration"]["root_module"]["resources"][4]["expressions"]["api_stages"]
                    [0]["stage"] = json!({"constant_value":"prod"})
            }
            _ => {
                bad["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.other")
            }
        }
        assert!(!verified(&bad, "api_stages.stage"), "{scenario}");
        assert!(!verified(&bad, "api_stages.api_id"), "{scenario}");
    }
    for scenario in 0..5 {
        let mut bad = v.clone();
        match scenario {
            0 => bad["resource_changes"][6]["change"]["after"]["key_type"] = json!("OTHER"),
            1 => bad["resource_changes"][6]["change"]["after_unknown"]["key_type"] = json!(true),
            2 => bad["resource_changes"][5]["change"]["after"]["id"] = json!("other"),
            3 => {
                bad["configuration"]["root_module"]["resources"][6]["expressions"]["key_id"] =
                    json!({"constant_value":"key"})
            }
            _ => bad["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
        }
        assert!(!verified(&bad, "key_id"), "{scenario}");
    }
}
#[test]
fn usage_plan_filtering_states_and_formats_never_expose_key_values() {
    let v = input();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(2);
    assert!(!verified(&filtered, "api_stages.stage"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: filtered.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_KEY_VALUE"));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
