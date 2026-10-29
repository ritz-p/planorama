use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str, endpoint: &str, output: &str) -> Value {
    let mut v = json!({"format_version":"1.2", "resource_changes":[], "configuration":{
        "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
        "root_module":{"resources":[]}}});
    for (resource, values, expressions) in [
        (
            endpoint,
            json!({output:"endpoint", "vpc_id":"vpc-main", "vpc_endpoint_type":"GatewayLoadBalancer"}),
            json!({}),
        ),
        (
            "aws_route_table",
            json!({"id":"table","vpc_id":"vpc-main"}),
            json!({}),
        ),
        (
            kind,
            json!({field:"endpoint", "route_table_id":"table"}),
            json!({field:{"references":[format!("{endpoint}.endpoint.{output}")]}, "route_table_id":{"references":["aws_route_table.table.id"]}}),
        ),
    ] {
        let name = if v["resource_changes"].as_array().unwrap().is_empty() {
            "endpoint"
        } else if v["resource_changes"].as_array().unwrap().len() == 1 {
            "table"
        } else {
            "main"
        };
        let address = format!("{resource}.{name}");
        v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":resource,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
        v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":resource,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
    }
    v
}

fn verified(v: &Value, field: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    raw.attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn typed_route_targets_and_prefix_list_navigation() {
    for (field, kind, output) in [
        ("vpc_endpoint_id", "aws_vpc_endpoint", "id"),
        ("core_network_arn", "aws_networkmanager_core_network", "arn"),
        ("odb_network_arn", "aws_odb_network", "arn"),
        (
            "destination_prefix_list_id",
            "aws_ec2_managed_prefix_list",
            "id",
        ),
    ] {
        let v = input("aws_route", field, kind, output);
        assert!(verified(&v, field), "{field}");
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 3);
        let route = g
            .nodes
            .iter()
            .find(|n| n.address == "aws_route.main")
            .unwrap();
        if field != "destination_prefix_list_id" {
            assert_eq!(
                route.routes.entries[0].target.as_ref().unwrap().address,
                format!("{kind}.endpoint")
            );
        }
        for case in 0..12 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][2]["change"]["after"][field] = "mismatch".into(),
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                2 => bad["resource_changes"][2]["change"]["after_unknown"] = json!({field:true}),
                3 => {
                    bad["configuration"]["root_module"]["resources"][2]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => {
                    bad["configuration"]["root_module"]["resources"][2]["expressions"][field] =
                        json!({"references":[format!("{kind}.endpoint.{output}"),"var.missing"]})
                }
                5 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => bad["resource_changes"][0]["deposed"] = "old".into(),
                7 => bad["resource_changes"][0]["mode"] = "data".into(),
                8 => bad["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
                9 => {
                    bad["configuration"]["root_module"]["resources"][2]["expressions"][field] =
                        json!({"references":[format!("{kind}.endpoint.name")]})
                }
                10 => bad["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
                _ => {
                    bad["configuration"]["provider_config"]["other"] =
                        bad["configuration"]["provider_config"]["aws"].clone();
                    bad["configuration"]["provider_config"]["other"]["expressions"]["region"]["constant_value"] =
                        "us-west-2".into();
                    bad["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                        "other".into();
                }
            }
            assert!(!verified(&bad, field), "{field} case {case}");
        }
    }
}

#[test]
fn endpoint_route_rejects_non_gwlb_modes() {
    for mode in [
        Value::Null,
        json!("Gateway"),
        json!("Interface"),
        json!("Resource"),
        json!("ServiceNetwork"),
    ] {
        let mut v = input("aws_route", "vpc_endpoint_id", "aws_vpc_endpoint", "id");
        v["resource_changes"][0]["change"]["after"]["vpc_endpoint_type"] = mode;
        assert!(!verified(&v, "vpc_endpoint_id"));
    }
}

#[test]
fn association_pairs_require_matching_values_vpc_and_current_endpoints() {
    for (helper, field, endpoint) in [
        ("aws_route_table_association", "subnet_id", "aws_subnet"),
        (
            "aws_route_table_association",
            "gateway_id",
            "aws_internet_gateway",
        ),
        (
            "aws_route_table_association",
            "gateway_id",
            "aws_vpn_gateway",
        ),
        ("aws_main_route_table_association", "vpc_id", "aws_vpc"),
    ] {
        let mut v = input(helper, field, endpoint, "id");
        if field == "vpc_id" {
            v["resource_changes"][1]["change"]["after"]["vpc_id"] = "endpoint".into();
        }
        for case in 0..8 {
            let mut v = v.clone();
            match case {
                1 => v["resource_changes"][2]["change"]["after"][field] = "wrong".into(),
                2 => v["resource_changes"][1]["change"]["after"]["vpc_id"] = "other".into(),
                3 => {
                    v["resource_changes"][2]["change"]["after_unknown"] =
                        json!({"route_table_id":true})
                }
                4 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                5 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][2]["deposed"] = "old".into(),
                7 => {
                    v["configuration"]["root_module"]["resources"][2]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                _ => (),
            }
            let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let lowered = !g.nodes.iter().any(|n| n.resource_type == helper);
            assert_eq!(lowered, case == 0, "{helper} {field} {case}");
            assert_eq!(g.coverage.selected.resources, 3);
        }
    }
}

#[test]
fn propagation_collection_must_be_complete() {
    let mut v = input(
        "aws_route_table",
        "propagating_vgws",
        "aws_vpn_gateway",
        "id",
    );
    v["resource_changes"][2]["change"]["after"]["propagating_vgws"] = json!(["endpoint"]);
    assert!(verified(&v, "propagating_vgws"));
    v["resource_changes"][2]["change"]["after"]["propagating_vgws"] =
        json!(["endpoint", "literal"]);
    assert!(!verified(&v, "propagating_vgws"));
}

#[test]
fn inline_new_targets_require_explicit_known_typed_references() {
    for (field, kind, output) in [
        ("vpc_endpoint_id", "aws_vpc_endpoint", "id"),
        ("core_network_arn", "aws_networkmanager_core_network", "arn"),
        ("odb_network_arn", "aws_odb_network", "arn"),
    ] {
        let mut v = input("aws_route_table", field, kind, output);
        v["resource_changes"][2]["change"]["after"] =
            json!({"route":[{field:"endpoint", "cidr_block":"0.0.0.0/0"}]});
        v["configuration"]["root_module"]["resources"][2]["expressions"] = json!({"route":[{field:{"references":[format!("{kind}.endpoint.{output}")]},"cidr_block":{"constant_value":"0.0.0.0/0"}}]});
        for case in 0..5 {
            let mut v = v.clone();
            match case {
                1 => {
                    v["resource_changes"][2]["change"]["after"]["route"][0][field] =
                        "mismatch".into()
                }
                2 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"route":true}),
                3 => {
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["route"][0]
                        [field] = json!({"constant_value":"endpoint"})
                }
                4 => v["resource_changes"][2]["change"]["after_sensitive"] = json!({"route":true}),
                _ => (),
            }
            let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let table = g
                .nodes
                .iter()
                .find(|n| n.address == "aws_route_table.main")
                .unwrap();
            assert_eq!(
                table.routes.entries[0].target.is_some(),
                case == 0,
                "{field} {case}"
            );
        }
    }
}

#[test]
fn route_targets_keep_filtering_named_state_and_safe_output_boundaries() {
    let v = input("aws_route", "odb_network_arn", "aws_odb_network", "arn");
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_route.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        selected
            .nodes
            .iter()
            .any(|n| n.resource_type == "aws_route_table")
    );
    assert!(
        selected
            .nodes
            .iter()
            .all(|n| n.resource_type != "aws_odb_network")
    );
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
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
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
