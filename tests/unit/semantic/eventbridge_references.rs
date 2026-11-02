use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/eventbridge-references-plan.json");

fn complete(value: &Value, index: usize, field: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let address = value["resource_changes"][index]["address"]
        .as_str()
        .unwrap();
    graph
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && super::label(
                    value["resource_changes"][index]["type"].as_str().unwrap(),
                    field,
                ) == Some(r.label)
        })
}

fn set_path(value: &mut Value, path: &[&str], replacement: Value) {
    if value.is_array() {
        set_path(&mut value[0], path, replacement);
    } else if let Some((field, rest)) = path.split_first() {
        set_path(&mut value[*field], rest, replacement);
    } else {
        *value = replacement;
    }
}

#[test]
fn references_require_known_unique_current_compatible_endpoints() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (index, field, endpoint) in [
        (1, "event_bus_name", 0),
        (3, "event_bus_name", 2),
        (5, "role_arn", 4),
        (7, "event_bus_name", 6),
        (9, "event_bus_name", 8),
        (11, "event_source_arn", 10),
        (13, "connection_arn", 12),
        (15, "role_arn", 14),
        (17, "dead_letter_config.arn", 16),
    ] {
        assert!(complete(&fixture, index, field), "{field}");
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "ambiguous",
            "wrong-type",
            "wrong-provider",
            "deleted",
            "consumer-deleted",
            "deposed",
            "data",
            "region",
            "unresolved-region",
            "mismatch",
            "wrong-field",
            "mask-shape",
            "account",
            "duplicate",
            "missing",
        ] {
            let mut v = fixture.clone();
            let path = field.split('.').collect::<Vec<_>>();
            let address = v["resource_changes"][endpoint]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                "literal" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"constant_value":"PRIVATE_VALUE"}),
                ),
                "unknown" => {
                    v["resource_changes"][index]["change"]["after_unknown"] = json!(true);
                }
                "source-unknown" => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!(true);
                }
                "ambiguous" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":[format!("{address}.arn"),"var.unknown"]}),
                ),
                "wrong-type" => v["resource_changes"][endpoint]["type"] = json!("aws_s3_bucket"),
                "wrong-provider" => {
                    v["resource_changes"][endpoint]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "deleted" => {
                    v["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"])
                }
                "consumer-deleted" => {
                    v["resource_changes"][index]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][endpoint]["deposed"] = json!("old"),
                "data" => v["resource_changes"][endpoint]["mode"] = json!("data"),
                "region" => {
                    let mut config = v["configuration"]["provider_config"]["aws"].clone();
                    config["expressions"]["region"] = json!({"constant_value":"us-west-2"});
                    v["configuration"]["provider_config"]["aws.other"] = config;
                    v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                        json!("aws.other");
                }
                "unresolved-region" => {
                    v["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                "mismatch" => set_path(
                    &mut v["resource_changes"][index]["change"]["after"],
                    &path,
                    json!("PRIVATE_OTHER"),
                ),
                "wrong-field" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":[format!("{address}.wrong")]}),
                ),
                "account" => {
                    let mut config = v["configuration"]["provider_config"]["aws"].clone();
                    config["expressions"]["assume_role"] = json!([{"role_arn":{"constant_value":"arn:aws:iam::999988887777:role/example"}}]);
                    v["configuration"]["provider_config"]["aws.other"] = config;
                    v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                        json!("aws.other");
                }
                "duplicate" => {
                    let output = v["resource_changes"][endpoint]["change"]["after"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .next()
                        .unwrap()
                        .to_owned();
                    let mut copy = v["resource_changes"][endpoint].clone();
                    copy["address"] = json!(format!("{address}_other"));
                    v["resource_changes"].as_array_mut().unwrap().push(copy);
                    set_path(
                        &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                        &path,
                        json!({"references":[format!("{address}.{output}"),format!("{address}_other.{output}")]}),
                    );
                }
                "missing" => {
                    v["resource_changes"][endpoint]["address"] =
                        json!("aws_cloudwatch_event_bus.absent");
                }
                "mask-shape" => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!([false])
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, index, field), "{field}: {case}");
        }
    }
}

#[test]
fn references_preserve_identity_selection_and_state_boundaries() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in (1..18).step_by(2) {
        let address = v["resource_changes"][index]["address"]
            .as_str()
            .unwrap()
            .to_owned();
        v["resource_changes"][index]["previous_address"] = json!(format!("{address}_old"));
        v["resource_changes"][index]["change"]["actions"] = json!(["update"]);
        v["resource_changes"][index]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    }
    let parsed = plan::parse(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 18);
    for source in &parsed.nodes {
        let nodes = graph
            .nodes
            .iter()
            .filter(|n| n.address == source.address)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
        assert!(nodes[0].member_owner.is_none());
    }
    let address = "aws_cloudwatch_event_api_destination.consumer_6";
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some(address.into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(12);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(out.contains("API destination connection"));
        assert!(out.contains("Event dead letter queue"));
    }
}

#[test]
fn current_endpoints_remain_valid_beside_deposed_instances() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in [0, 1] {
        let mut old = v["resource_changes"][index].clone();
        old["deposed"] = json!("old");
        old["change"]["actions"] = json!(["delete"]);
        v["resource_changes"].as_array_mut().unwrap().push(old);
    }
    assert!(complete(&v, 1, "event_bus_name"));
    let mut scheduler: Value = serde_json::from_str(include_str!(
        "../../fixtures/eventbridge-configuration-plan.json"
    ))
    .unwrap();
    let mut old = scheduler["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    scheduler["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(old);
    assert!(complete(&scheduler, 5, "group_name"));
}
