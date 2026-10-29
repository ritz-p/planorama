use crate::model::{EdgeKind, Placement};
use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str, endpoint: &str, output: &str) -> Value {
    let mut v = json!({"format_version":"1.2", "resource_changes":[], "configuration":{
        "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
        "root_module":{"resources":[]}}});
    add(
        &mut v,
        endpoint,
        "endpoint",
        json!({output:"endpoint", "domain":"vpc", "availability_mode":"zonal", "connectivity_type":"public"}),
        json!({}),
    );
    add(
        &mut v,
        kind,
        "main",
        json!({field:"endpoint", "domain":"vpc", "availability_mode":"zonal", "connectivity_type":"public"}),
        json!({field:{"references":[format!("{endpoint}.endpoint.{output}")]}}),
    );
    v
}

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn gateway_eip_typed_references_and_negative_evidence() {
    for (kind, field, endpoint, output) in [
        (
            "aws_egress_only_internet_gateway",
            "vpc_id",
            "aws_vpc",
            "id",
        ),
        ("aws_internet_gateway", "vpc_id", "aws_vpc", "id"),
        ("aws_eip", "instance", "aws_instance", "id"),
        (
            "aws_eip",
            "network_interface",
            "aws_network_interface",
            "id",
        ),
        ("aws_eip", "ipam_pool_id", "aws_vpc_ipam_pool", "id"),
        (
            "aws_eip_domain_name",
            "allocation_id",
            "aws_eip",
            "allocation_id",
        ),
        (
            "aws_nat_gateway",
            "allocation_id",
            "aws_eip",
            "allocation_id",
        ),
    ] {
        let v = input(kind, field, endpoint, output);
        assert!(verified(&v, field), "{kind} {field}");
        for case in 0..12 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][1]["change"]["after"][field] = "other".into(),
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                2 => bad["resource_changes"][1]["change"]["after_unknown"] = json!({field:true}),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{endpoint}.endpoint.{output}"),"var.unresolved"]})
                }
                5 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => bad["resource_changes"][0]["deposed"] = "old".into(),
                7 => bad["resource_changes"][0]["mode"] = "data".into(),
                8 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                9 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{endpoint}.endpoint.association_id")]})
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
            assert!(!verified(&bad, field), "{kind} {field} case {case}");
        }
    }
}

#[test]
fn eip_discriminators_and_reverse_dns_owner() {
    let public_pool = input("aws_eip", "public_ipv4_pool", "aws_vpc_ipam_pool", "id");
    assert!(!verified(&public_pool, "public_ipv4_pool"));
    let mut v = input("aws_eip", "instance", "aws_instance", "id");
    v["resource_changes"][1]["change"]["after"]["network_interface"] = "eni-computed".into();
    assert!(!verified(&v, "instance"));
    v["resource_changes"][1]["change"]["after"]["network_interface"] = Value::Null;
    v["resource_changes"][1]["change"]["after_unknown"] = json!({"network_interface":true});
    assert!(!verified(&v, "instance"));
    let v = input(
        "aws_eip_domain_name",
        "allocation_id",
        "aws_eip",
        "allocation_id",
    );
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(g.nodes[1].member_owner.is_some());
    assert_eq!(g.coverage.selected.resources, 2);
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
}

#[test]
fn named_gateway_outputs_and_filtered_fallback_are_safe() {
    let mut v = input(
        "aws_eip_domain_name",
        "allocation_id",
        "aws_eip",
        "allocation_id",
    );
    v["resource_changes"][1]["change"]["after"]["domain_name"] =
        "<script>private-ptr</script>".into();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!verified(&filtered, "allocation_id"));
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
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
        assert!(!out.contains("private-ptr"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}

#[test]
fn gateway_context_and_nat_mode_preserve_legacy_subnet_contract() {
    let v = input(
        "aws_egress_only_internet_gateway",
        "vpc_id",
        "aws_vpc",
        "id",
    );
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.nodes[1].placement, Placement::Spatial);
    assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    let mut data = v.clone();
    data["resource_changes"][1]["mode"] = "data".into();
    let data = semantic::transform(&plan::parse(&data.to_string()).unwrap());
    assert_eq!(
        crate::provider::aws::data_placement("aws_egress_only_internet_gateway"),
        Placement::Unresolved
    );
    assert!(!data.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    let mut v = input("aws_nat_gateway", "vpc_id", "aws_vpc", "id");
    v["resource_changes"][1]["change"]["after"]["availability_mode"] = "regional".into();
    assert!(verified(&v, "vpc_id"));
    assert!(
        semantic::transform(&plan::parse(&v.to_string()).unwrap())
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment)
    );
    v["resource_changes"][1]["change"]["after"]["connectivity_type"] = "private".into();
    assert!(!verified(&v, "vpc_id"));
    let mut v = input("aws_nat_gateway", "subnet_id", "aws_subnet", "id");
    for (mode, expected) in [
        (json!("zonal"), true),
        (Value::Null, true),
        (json!("regional"), false),
        (json!("bad"), false),
    ] {
        v["resource_changes"][1]["change"]["after"]["availability_mode"] = mode;
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(
            g.edges.iter().any(|e| e.kind == EdgeKind::Containment),
            expected
        );
    }
    v["resource_changes"][1]["change"]["after_unknown"] = json!({"availability_mode":true});
    assert!(!verified(&v, "subnet_id"));
}

#[test]
fn nat_allocation_sets_require_complete_unique_known_evidence() {
    for field in [
        "secondary_allocation_ids",
        "availability_zone_address.allocation_ids",
    ] {
        let mut v = input("aws_nat_gateway", field, "aws_eip", "allocation_id");
        if field.starts_with("availability") {
            v["resource_changes"][1]["change"]["after"]["availability_mode"] = "regional".into();
            v["resource_changes"][1]["change"]["after"]["availability_zone_address"] =
                json!([{"availability_zone_id":"use1-az1","allocation_ids":["endpoint"]}]);
        } else {
            v["resource_changes"][1]["change"]["after"][field] = json!(["endpoint"]);
        }
        assert!(verified(&v, field), "{field}");
        for case in 0..4 {
            let mut bad = v.clone();
            if field.starts_with("availability") {
                match case {
                    0 => {
                        bad["resource_changes"][1]["change"]["after"]["availability_zone_address"]
                            [0]["allocation_ids"] = json!(["endpoint", "literal"])
                    }
                    1 => {
                        bad["resource_changes"][1]["change"]["after_unknown"] =
                            json!({"availability_zone_address":true})
                    }
                    2 => {
                        bad["resource_changes"][1]["change"]["after"]["availability_zone_address"]
                            [0]["availability_zone_id"] = Value::Null
                    }
                    _ => {
                        bad["resource_changes"][1]["change"]["after"]["availability_zone_address"]
                            [0]["allocation_ids"] = json!(["endpoint", "endpoint"])
                    }
                }
            } else {
                match case {
                    0 => {
                        bad["resource_changes"][1]["change"]["after"][field] =
                            json!(["endpoint", "literal"])
                    }
                    1 => {
                        bad["resource_changes"][1]["change"]["after_unknown"] = json!({field:true})
                    }
                    2 => {
                        bad["resource_changes"][1]["change"]["after"]["connectivity_type"] =
                            "private".into()
                    }
                    _ => {
                        bad["resource_changes"][1]["change"]["after"][field] =
                            json!(["endpoint", "endpoint"])
                    }
                }
            }
            assert!(!verified(&bad, field), "{field} case {case}");
        }
    }
}

#[test]
fn gateway_association_pairs_preserve_resource_action_coverage() {
    for (kind, field, endpoint, other_field, other_kind, other_output) in [
        (
            "aws_eip_association",
            "instance_id",
            "aws_instance",
            "allocation_id",
            "aws_eip",
            "allocation_id",
        ),
        (
            "aws_eip_association",
            "network_interface_id",
            "aws_network_interface",
            "allocation_id",
            "aws_eip",
            "allocation_id",
        ),
        (
            "aws_nat_gateway_eip_association",
            "nat_gateway_id",
            "aws_nat_gateway",
            "allocation_id",
            "aws_eip",
            "allocation_id",
        ),
        (
            "aws_internet_gateway_attachment",
            "internet_gateway_id",
            "aws_internet_gateway",
            "vpc_id",
            "aws_vpc",
            "id",
        ),
    ] {
        let mut v = input(kind, field, endpoint, "id");
        add(
            &mut v,
            other_kind,
            "other",
            json!({other_output:"other","domain":"vpc"}),
            json!({}),
        );
        v["resource_changes"][1]["change"]["after"][other_field] = "other".into();
        v["configuration"]["root_module"]["resources"][1]["expressions"][other_field] =
            json!({"references":[format!("{other_kind}.other.{other_output}")]});
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.coverage.selected.resources, 3);
        assert_eq!(g.nodes.len(), 2, "{kind} {field}");
        if kind == "aws_internet_gateway_attachment" {
            assert!(
                !semantic::diagnostics::collect(&plan::parse(&v.to_string()).unwrap())
                    .iter()
                    .any(|d| d.address == "aws_internet_gateway.endpoint")
            );
        }
        assert_eq!(
            g.edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Association)
                .count(),
            1
        );
        if kind == "aws_nat_gateway_eip_association" {
            for (key, value) in [
                ("availability_mode", json!("regional")),
                ("availability_mode", Value::Null),
                ("connectivity_type", json!("private")),
            ] {
                let mut bad = v.clone();
                bad["resource_changes"][0]["change"]["after"][key] = value;
                let graph = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
                assert_eq!(graph.nodes.len(), 3);
            }
        }
        v["resource_changes"][2]["change"]["after"][other_output] = "mismatch".into();
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.coverage.selected.resources, 3);
    }
}

#[test]
fn internet_gateway_only_diagnoses_configured_vpc_references() {
    let mut v = input("aws_internet_gateway", "vpc_id", "aws_vpc", "id");
    let diagnostics = |v: &Value| {
        semantic::diagnostics::collect(&plan::parse(&v.to_string()).unwrap())
            .into_iter()
            .filter(|d| d.address == "aws_internet_gateway.main" && d.attribute == "vpc_id")
            .collect::<Vec<_>>()
    };
    assert!(diagnostics(&v).is_empty());
    v["resource_changes"][1]["change"]["after"]["vpc_id"] = "mismatch".into();
    assert!(!diagnostics(&v).is_empty());
    v["resource_changes"][1]["change"]["after"] = json!({"id":"igw-main"});
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({});
    assert!(diagnostics(&v).is_empty());
}
