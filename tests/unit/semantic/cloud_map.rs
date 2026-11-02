use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str) -> Value {
    let collection = field == "subnet_ids";
    let endpoint = if field == "network_acl_id" {
        "aws_network_acl"
    } else {
        "aws_subnet"
    };
    json!({"format_version":"1.2", "resource_changes":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"id":"endpoint","vpc_id":"vpc-one"}}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{field:if collection {json!(["endpoint"])} else {json!("endpoint")},"vpc_id":"vpc-one","rule_number":100,"egress":false,"protocol":"tcp","rule_action":"allow"}}}
    ],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_config_key":"aws","expressions":{}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_config_key":"aws","expressions":{field:{"references":[format!("{endpoint}.peer.id")]}}}
    ]}}})
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn cloud_map_references_require_current_explicit_matching_outputs() {
    for (kind, field, endpoint, output) in [
        (
            "aws_service_discovery_private_dns_namespace",
            "vpc",
            "aws_vpc",
            "id",
        ),
        (
            "aws_service_discovery_service",
            "namespace_id",
            "aws_service_discovery_http_namespace",
            "id",
        ),
        (
            "aws_service_discovery_instance",
            "service_id",
            "aws_service_discovery_service",
            "id",
        ),
    ] {
        let mut v = input(kind, field);
        v["resource_changes"][1]["change"]["after"]["type"] = json!("HTTP");
        v["resource_changes"][0]["type"] = json!(endpoint);
        v["resource_changes"][0]["address"] = json!(format!("{endpoint}.peer"));
        v["resource_changes"][0]["change"]["after"] = json!({output:"endpoint"});
        v["configuration"]["root_module"]["resources"][0]["type"] = json!(endpoint);
        v["configuration"]["root_module"]["resources"][0]["address"] =
            json!(format!("{endpoint}.peer"));
        v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
            json!({"references":[format!("{endpoint}.peer.{output}")]});
        assert!(verified(&v, field), "{kind} {field}");
        if kind == "aws_service_discovery_service" {
            v["resource_changes"][1]["change"]["after"]["dns_config"] = json!([]);
            v["resource_changes"][1]["change"]["after_unknown"]["dns_config"] = json!([]);
            assert!(verified(&v, field));
            for mask in [json!(true), json!([{}]), json!({}), json!([true])] {
                let mut bad = v.clone();
                bad["resource_changes"][1]["change"]["after_unknown"]["dns_config"] = mask;
                assert!(!verified(&bad, field));
            }
        }
        assert_eq!(
            semantic::transform(&plan::parse(&v.to_string()).unwrap())
                .coverage
                .selected
                .resources,
            2
        );
        for scenario in 0..8 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"][output] = json!("mismatch"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"][output] = json!(true),
                2 => bad["resource_changes"][1]["change"]["after_unknown"][field] = json!(true),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => bad["resource_changes"][0]["mode"] = json!("data"),
                5 => bad["resource_changes"][0]["deposed"] = json!("old"),
                6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                _ => {
                    bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.other")
                }
            }
            assert!(!verified(&bad, field), "{kind} {field}: {scenario}");
        }
    }
}

#[test]
fn dns_discriminators_and_alternative_namespaces_must_agree_without_containment() {
    let v: Value =
        serde_json::from_str(include_str!("../../fixtures/cloud-map-plan.json")).unwrap();
    assert!(verified(&v, "dns_config.namespace_id"));
    for (mask, expected) in [
        (json!([{}]), true),
        (json!([{"namespace_id":false,"dns_records":true}]), true),
        (json!([{"namespace_id":true}]), false),
        (json!([true]), false),
        (json!([]), false),
        (json!([{}, {}]), false),
    ] {
        let mut masked = v.clone();
        masked["resource_changes"][2]["change"]["after_unknown"]["dns_config"] = mask;
        assert_eq!(verified(&masked, "dns_config.namespace_id"), expected);
        masked["resource_changes"][2]["change"]["after"]["namespace_id"] = json!("ns-main");
        masked["configuration"]["root_module"]["resources"][2]["expressions"]["namespace_id"] =
            json!({"references":["aws_service_discovery_private_dns_namespace.main.id"]});
        assert_eq!(verified(&masked, "namespace_id"), expected);
    }
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 5);
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for scenario in 0..5 {
        let mut bad = v.clone();
        match scenario {
            0 => bad["resource_changes"][2]["change"]["after"]["namespace_id"] = json!("other"),
            1 => bad["resource_changes"][2]["change"]["after"]["type"] = json!("HTTP"),
            2 => bad["resource_changes"][2]["change"]["after_unknown"]["type"] = json!(true),
            3 => bad["resource_changes"][2]["change"]["after_unknown"]["dns_config"] = json!(true),
            _ => {
                bad["resource_changes"][2]["change"]["after"]["dns_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"namespace_id":"ns-main"}));
            }
        }
        assert!(!verified(&bad, "dns_config.namespace_id"), "{scenario}");
    }
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(!verified(&filtered, "dns_config.namespace_id"));
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
        assert!(!out.contains("PRIVATE_IP"));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
