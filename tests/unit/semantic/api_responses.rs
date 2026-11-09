use crate::{plan, semantic};
use serde_json::{Value, json};
fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/api-responses-plan.json")).unwrap()
}
fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}
#[test]
fn websocket_responses_require_same_api_and_complete_model_names() {
    let v = input();
    for field in ["integration_id", "route_id", "response_models"] {
        assert!(verified(&v, field), "{field}");
    }
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 6);
    for (field, endpoint, consumer) in [
        ("integration_id", 1, 4),
        ("route_id", 2, 5),
        ("response_models", 3, 5),
    ] {
        for scenario in 0..11 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"]["protocol_type"] = json!("HTTP"),
                1 => {
                    bad["resource_changes"][endpoint]["change"]["after"]["api_id"] = json!("other")
                }
                2 => {
                    bad["resource_changes"][endpoint]["change"]["after_unknown"]
                        [if endpoint == 3 { "name" } else { "id" }] = json!(true)
                }
                3 => {
                    bad["resource_changes"][consumer]["change"]["after_unknown"][field] =
                        json!(true)
                }
                4 => {
                    bad["configuration"]["root_module"]["resources"][consumer]["expressions"]
                        [field] = json!({"constant_value":"literal"})
                }
                5 => bad["resource_changes"][endpoint]["mode"] = json!("data"),
                6 => bad["resource_changes"][endpoint]["deposed"] = json!("old"),
                7 => bad["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"]),
                8 => bad["resource_changes"][consumer]["change"]["actions"] = json!(["delete"]),
                9 => {
                    bad["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                        json!("aws.other")
                }
                _ => {
                    bad["resource_changes"][consumer]["change"]["after_unknown"][if consumer == 4 {
                        "integration_response_key"
                    } else {
                        "route_response_key"
                    }] = json!(true)
                }
            }
            assert!(!verified(&bad, field), "{field}: {scenario}");
        }
    }
    let mut bad = v.clone();
    bad["resource_changes"][5]["change"]["after"]["response_models"]["extra"] = json!("missing");
    assert!(!verified(&bad, "response_models"));
    let mut bad = v.clone();
    bad["resource_changes"][2]["change"]["after"]["api_id"] = json!("other");
    assert!(!verified(&bad, "response_models"));
}
#[test]
fn response_formats_filtering_and_named_states_preserve_identity_without_bodies() {
    let v = input();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(3);
    assert!(!verified(&filtered, "response_models"));
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
        assert!(!out.contains("PRIVATE_SCHEMA"));
        assert!(!out.contains("PRIVATE_TEMPLATE"));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
