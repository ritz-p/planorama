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
fn adoption_requires_explicit_matching_vpc_output_and_current_identity() {
    for (kind, field) in [
        ("aws_default_network_acl", "default_network_acl_id"),
        ("aws_default_route_table", "default_route_table_id"),
        ("aws_default_security_group", "vpc_id"),
    ] {
        for endpoint in ["aws_vpc", "aws_default_vpc"] {
            let mut v = input(kind, field);
            let output = if field == "vpc_id" { "id" } else { field };
            v["resource_changes"][0]["type"] = json!(endpoint);
            v["resource_changes"][0]["address"] = json!(format!("{endpoint}.peer"));
            v["resource_changes"][0]["change"]["after"] = json!({"id":"vpc-one", field:"endpoint"});
            if field == "vpc_id" {
                v["resource_changes"][0]["change"]["after"]["id"] = json!("endpoint");
            }
            v["resource_changes"][1]["change"]["after"][field] = json!("endpoint");
            v["resource_changes"][1]["change"]["after"]["id"] = json!("endpoint");
            v["configuration"]["root_module"]["resources"][0]["type"] = json!(endpoint);
            v["configuration"]["root_module"]["resources"][0]["address"] =
                json!(format!("{endpoint}.peer"));
            v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                json!({"references":[format!("{endpoint}.peer.{output}")]});
            assert!(verified(&v, field), "{kind} {endpoint}");
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            assert_eq!(graph.coverage.selected.resources, 2);
            assert!(
                graph
                    .edges
                    .iter()
                    .any(|e| e.kind == crate::model::EdgeKind::Containment)
            );
            for scenario in 0..9 {
                let mut bad = v.clone();
                match scenario {
                    0 => bad["resource_changes"][0]["change"]["after"][output] = json!("mismatch"),
                    1 => {
                        bad["resource_changes"][0]["change"]["after_unknown"][output] = json!(true)
                    }
                    2 => bad["resource_changes"][1]["change"]["after_unknown"][field] = json!(true),
                    3 => {
                        bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                            json!({"constant_value":"endpoint"})
                    }
                    4 => bad["resource_changes"][0]["mode"] = json!("data"),
                    5 => bad["resource_changes"][0]["deposed"] = json!("old"),
                    6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                    7 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                    _ => {
                        bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                            json!("aws.other")
                    }
                }
                assert!(!verified(&bad, field), "{kind}: {scenario}");
            }
        }
    }
}

#[test]
fn computed_default_subnet_vpc_does_not_establish_containment() {
    let mut v = input("aws_default_subnet", "vpc_id");
    v["resource_changes"][0]["type"] = json!("aws_default_vpc");
    v["resource_changes"][0]["address"] = json!("aws_default_vpc.peer");
    v["configuration"]["root_module"]["resources"][0]["type"] = json!("aws_default_vpc");
    v["configuration"]["root_module"]["resources"][0]["address"] = json!("aws_default_vpc.peer");
    v["configuration"]["root_module"]["resources"][1]["expressions"] =
        json!({"availability_zone":{"constant_value":"us-east-1a"}});
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 2);
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.role == crate::model::ResourceRole::Container)
    );
}

#[test]
fn default_route_targets_require_complete_current_typed_evidence() {
    for (field, endpoint) in [
        ("route.nat_gateway_id", "aws_nat_gateway"),
        ("propagating_vgws", "aws_vpn_gateway"),
    ] {
        let mut v = input("aws_default_route_table", field);
        v["resource_changes"][0]["type"] = json!(endpoint);
        v["resource_changes"][0]["address"] = json!(format!("{endpoint}.peer"));
        v["configuration"]["root_module"]["resources"][0]["type"] = json!(endpoint);
        v["configuration"]["root_module"]["resources"][0]["address"] =
            json!(format!("{endpoint}.peer"));
        v["configuration"]["root_module"]["resources"][1]["expressions"] =
            if field.starts_with("route.") {
                json!({"route":[{"nat_gateway_id":{"references":[format!("{endpoint}.peer.id")]}}]})
            } else {
                json!({field:{"references":[format!("{endpoint}.peer.id")]}})
            };
        v["resource_changes"][1]["change"]["after"] = if field.starts_with("route.") {
            json!({"route":[{"nat_gateway_id":"endpoint"}]})
        } else {
            json!({field:["endpoint"]})
        };
        assert!(verified(&v, field));
        for scenario in 0..4 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"]["id"] = json!("mismatch"),
                1 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                2 => bad["resource_changes"][1]["change"]["after_unknown"] = json!(true),
                _ => {
                    if field.starts_with("route.") {
                        bad["resource_changes"][1]["change"]["after"]["route"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"nat_gateway_id":"missing"}));
                    } else {
                        bad["resource_changes"][1]["change"]["after"][field] =
                            json!(["endpoint", "missing"]);
                    }
                }
            }
            assert!(!verified(&bad, field), "{field} {scenario}");
        }
    }
}

#[test]
fn default_tables_do_not_invent_unconfigured_computed_routes() {
    for kind in ["aws_default_route_table", "aws_route_table"] {
        for planned_only in [false, true] {
            for unknown in [json!({}), json!({"route": true})] {
                let address = format!("{kind}.main");
                let values = json!({"route":[{"cidr_block":"10.0.0.0/16","gateway_id":"local"}]});
                let mut v = json!({"format_version":"1.2",
                    "planned_values":{"root_module":{"resources":[{"address":address,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","values":values}]}},
                    "configuration":{"root_module":{"resources":[{"address":address,"mode":"managed","type":kind,"expressions":{}}]}}
                });
                if !planned_only {
                    v["resource_changes"] = json!([{"address":address,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["no-op"],"after":values,"after_unknown":unknown}}]);
                }
                let raw = plan::parse(&v.to_string()).unwrap();
                assert_eq!(raw.nodes.len(), 1);
                assert_eq!(
                    raw.nodes[0].route_entries.len(),
                    usize::from(kind == "aws_route_table")
                );
                if kind == "aws_default_route_table" {
                    let out = crate::generate(&v.to_string(), &crate::Options::default()).unwrap();
                    assert!(!out.content.contains("Inline route"));
                }
                v["configuration"]["root_module"]["resources"][0]["expressions"]["route"] =
                    json!([{"cidr_block":{"constant_value":"0.0.0.0/0"}}]);
                assert_eq!(
                    plan::parse(&v.to_string()).unwrap().nodes[0]
                        .route_entries
                        .len(),
                    1
                );
            }
        }
    }
}

#[test]
fn adopted_network_formats_and_named_states_keep_each_identity() {
    let v: Value =
        serde_json::from_str(include_str!("../../fixtures/default-network-plan.json")).unwrap();
    let raw = plan::parse(&v.to_string()).unwrap();
    assert_eq!(semantic::transform(&raw).coverage.selected.resources, 5);
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!verified(&filtered, "default_network_acl_id"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
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
        assert!(output.contains("data-state-id=\"one\""));
        assert!(output.contains("data-state-id=\"two\""));
    }
}
