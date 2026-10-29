use crate::model::EdgeKind;
use crate::{plan, semantic};
use serde_json::{Value, json};

fn base() -> Value {
    json!({"format_version":"1.2", "resource_changes":[], "configuration":{
        "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
        "root_module":{"resources":[]}}})
}

fn cross_accounts(v: &mut Value) {
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    v["configuration"]["provider_config"]["producer"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"},"assume_role":[{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]}});
}

#[test]
fn cross_account_service_region_and_accepter_keep_independent_scopes() {
    let mut v = input(
        "aws_vpc_endpoint",
        "service_name",
        "aws_vpc_endpoint_service",
        "service_name",
        false,
    );
    cross_accounts(&mut v);
    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] = "producer".into();
    v["resource_changes"][1]["change"]["after"]["service_region"] = "us-west-2".into();
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    assert!(
        parsed
            .attributes
            .iter()
            .any(|r| r.attribute == "service_name" && r.service_reference_verified)
    );
    let graph = semantic::transform(&parsed);
    assert_ne!(graph.nodes[0].entity.scope, graph.nodes[1].entity.scope);
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Containment));
    for region in [Value::Null, json!("us-east-1")] {
        let mut bad = v.clone();
        bad["resource_changes"][1]["change"]["after"]["service_region"] = region;
        let parsed = crate::plan::evidence::tests::parse_fixture(&bad.to_string()).unwrap();
        assert!(
            !parsed
                .attributes
                .iter()
                .any(|r| r.attribute == "service_name" && r.service_reference_verified)
        );
    }
    let mut v = pair(
        "aws_vpc_endpoint_connection_accepter",
        "vpc_endpoint_service_id",
        "aws_vpc_endpoint_service",
        "Interface",
    );
    cross_accounts(&mut v);
    for index in [1, 2] {
        v["configuration"]["root_module"]["resources"][index]["provider_config_key"] =
            "producer".into();
    }
    v["resource_changes"][0]["change"]["after"]["service_region"] = "us-west-2".into();
    let graph =
        semantic::transform(&crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
    assert_eq!(graph.nodes.len(), 2);
    assert_ne!(graph.nodes[0].entity.scope, graph.nodes[1].entity.scope);
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] = Value::Null;
    assert!(!verified(&v, "vpc_endpoint_id"));
}

fn add(v: &mut Value, kind: &str, name: &str, values: Value, expressions: Value) {
    let address = format!("{kind}.{name}");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"type":kind,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
}

fn input(kind: &str, field: &str, endpoint: &str, output: &str, collection: bool) -> Value {
    let mut v = base();
    let value = if endpoint == "aws_iam_role" {
        "arn:aws:iam::111111111111:role/reader"
    } else {
        "endpoint"
    };
    add(
        &mut v,
        endpoint,
        "endpoint",
        json!({output:value,"vpc_id":"vpc-main","vpc_endpoint_type":"Interface","service_type":"Interface","load_balancer_type":"network"}),
        json!({}),
    );
    add(
        &mut v,
        kind,
        "main",
        json!({field:if collection {json!([value])} else {json!(value)}, "vpc_id":"vpc-main","vpc_endpoint_type":"Interface","service_type":"Interface"}),
        json!({field:{"references":[format!("{endpoint}.endpoint.{output}")]}}),
    );
    v
}

fn verified(v: &Value, field: &str) -> bool {
    let parsed = plan::parse(&v.to_string()).unwrap();
    let raw = crate::semantic::Input::new(&parsed);
    raw.attributes
        .iter()
        .any(|r| r.attribute == field && super::super::service_references::verified(&raw, r))
}

#[test]
fn typed_endpoint_service_and_helper_contracts_require_complete_evidence() {
    for (kind, field, endpoint, output, collection) in [
        ("aws_vpc_endpoint", "subnet_ids", "aws_subnet", "id", true),
        (
            "aws_vpc_endpoint",
            "security_group_ids",
            "aws_security_group",
            "id",
            true,
        ),
        (
            "aws_vpc_endpoint",
            "service_name",
            "aws_vpc_endpoint_service",
            "service_name",
            false,
        ),
        (
            "aws_vpc_endpoint_service",
            "network_load_balancer_arns",
            "aws_lb",
            "arn",
            true,
        ),
        (
            "aws_vpc_endpoint_service",
            "allowed_principals",
            "aws_iam_role",
            "arn",
            true,
        ),
        (
            "aws_vpc_endpoint_service_allowed_principal",
            "principal_arn",
            "aws_iam_role",
            "arn",
            false,
        ),
        (
            "aws_vpc_endpoint_service_allowed_principal",
            "vpc_endpoint_service_id",
            "aws_vpc_endpoint_service",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_service_private_dns_verification",
            "service_id",
            "aws_vpc_endpoint_service",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_policy",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_private_dns",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_connection_notification",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_connection_notification",
            "vpc_endpoint_service_id",
            "aws_vpc_endpoint_service",
            "id",
            false,
        ),
        (
            "aws_vpc_endpoint_connection_notification",
            "connection_notification_arn",
            "aws_sns_topic",
            "arn",
            false,
        ),
    ] {
        let v = input(kind, field, endpoint, output, collection);
        assert!(verified(&v, field), "{kind} {field}");
        for case in 0..11 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][0]["change"]["after"][output] = "mismatch".into(),
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
                        json!({"references":[format!("{endpoint}.endpoint.wrong")]})
                }
                _ => bad["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            }
            assert!(!verified(&bad, field), "{kind} {field} case {case}");
        }
    }
}

#[test]
fn network_sets_gate_modes_vpc_and_nested_subnet_membership() {
    for mode in [
        "Gateway",
        "Interface",
        "GatewayLoadBalancer",
        "Resource",
        "ServiceNetwork",
        "unknown",
    ] {
        for (field, kind, allowed) in [
            (
                "subnet_ids",
                "aws_subnet",
                matches!(
                    mode,
                    "Interface" | "GatewayLoadBalancer" | "Resource" | "ServiceNetwork"
                ),
            ),
            (
                "security_group_ids",
                "aws_security_group",
                matches!(mode, "Interface" | "Resource" | "ServiceNetwork"),
            ),
            ("route_table_ids", "aws_route_table", mode == "Gateway"),
        ] {
            let mut v = input("aws_vpc_endpoint", field, kind, "id", true);
            v["resource_changes"][1]["change"]["after"]["vpc_endpoint_type"] = mode.into();
            assert_eq!(verified(&v, field), allowed, "{mode} {field}");
            v["resource_changes"][0]["change"]["after"]["vpc_id"] = "vpc-other".into();
            assert!(!verified(&v, field));
        }
    }
    let mut v = input(
        "aws_vpc_endpoint",
        "subnet_configuration.subnet_id",
        "aws_subnet",
        "id",
        true,
    );
    v["resource_changes"][1]["change"]["after"]["subnet_ids"] = json!(["endpoint"]);
    v["resource_changes"][1]["change"]["after"]["subnet_configuration"] =
        json!([{"subnet_id":"endpoint","ipv4":"PRIVATE_PAYLOAD"}]);
    assert!(verified(&v, "subnet_configuration.subnet_id"));
    v["resource_changes"][1]["change"]["after"]["subnet_ids"] = json!(["other"]);
    assert!(!verified(&v, "subnet_configuration.subnet_id"));
    v["resource_changes"][1]["change"]["after_unknown"] = json!({"subnet_configuration":true});
    assert!(!verified(&v, "subnet_configuration.subnet_id"));
}

#[test]
fn lattice_and_load_balancer_modes_and_principals_are_allowlisted() {
    for (field, kind, mode) in [
        (
            "resource_configuration_arn",
            "aws_vpclattice_resource_configuration",
            "Resource",
        ),
        (
            "service_network_arn",
            "aws_vpclattice_service_network",
            "ServiceNetwork",
        ),
    ] {
        let mut v = input("aws_vpc_endpoint", field, kind, "arn", false);
        v["resource_changes"][1]["change"]["after"]["vpc_endpoint_type"] = mode.into();
        assert!(verified(&v, field));
        v["resource_changes"][1]["change"]["after"]["service_name"] =
            "com.amazonaws.example".into();
        assert!(!verified(&v, field));
    }
    let mut v = input(
        "aws_vpc_endpoint_service",
        "gateway_load_balancer_arns",
        "aws_lb",
        "arn",
        true,
    );
    v["resource_changes"][0]["change"]["after"]["load_balancer_type"] = "gateway".into();
    v["resource_changes"][1]["change"]["after"]["service_type"] = "GatewayLoadBalancer".into();
    assert!(verified(&v, "gateway_load_balancer_arns"));
    v["resource_changes"][0]["change"]["after"]["load_balancer_type"] = "application".into();
    assert!(!verified(&v, "gateway_load_balancer_arns"));
    for principal in [
        "*",
        "arn:aws:iam::111111111111:root",
        "arn:aws:iam::111111111111:role/*",
        "arn:aws:sts::111111111111:assumed-role/reader/session",
    ] {
        let mut v = input(
            "aws_vpc_endpoint_service",
            "allowed_principals",
            "aws_iam_role",
            "arn",
            true,
        );
        v["resource_changes"][0]["change"]["after"]["arn"] = principal.into();
        v["resource_changes"][1]["change"]["after"]["allowed_principals"] = json!([principal]);
        assert!(!verified(&v, "allowed_principals"));
    }
}

fn pair(kind: &str, other_field: &str, other_kind: &str, mode: &str) -> Value {
    let mut v = input(kind, "vpc_endpoint_id", "aws_vpc_endpoint", "id", false);
    v["resource_changes"][0]["change"]["after"]["vpc_endpoint_type"] = mode.into();
    v["resource_changes"][0]["change"]["after"]["service_name"] = "service".into();
    add(
        &mut v,
        other_kind,
        "other",
        json!({"id":"other","vpc_id":"vpc-main","service_name":"service","service_type":mode}),
        json!({}),
    );
    v["resource_changes"][1]["change"]["after"][other_field] = "other".into();
    v["configuration"]["root_module"]["resources"][1]["expressions"][other_field] =
        json!({"references":[format!("{other_kind}.other.id")]});
    v
}

#[test]
fn associations_require_known_pairs_and_preserve_fallback_actions() {
    for (kind, field, endpoint, mode) in [
        (
            "aws_vpc_endpoint_route_table_association",
            "route_table_id",
            "aws_route_table",
            "Gateway",
        ),
        (
            "aws_vpc_endpoint_subnet_association",
            "subnet_id",
            "aws_subnet",
            "Interface",
        ),
        (
            "aws_vpc_endpoint_security_group_association",
            "security_group_id",
            "aws_security_group",
            "Interface",
        ),
        (
            "aws_vpc_endpoint_connection_accepter",
            "vpc_endpoint_service_id",
            "aws_vpc_endpoint_service",
            "Interface",
        ),
    ] {
        let v = pair(kind, field, endpoint, mode);
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.nodes.len(), 2, "{kind}");
        assert_eq!(g.coverage.selected.resources, 3);
        assert_eq!(
            g.edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Association)
                .count(),
            1
        );
        for case in 0..4 {
            let mut bad = v.clone();
            match case {
                0 => {
                    bad["resource_changes"][0]["change"]["after"]["vpc_endpoint_type"] =
                        "unknown".into()
                }
                1 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                2 => bad["resource_changes"][2]["change"]["after"]["id"] = "mismatch".into(),
                _ if kind == "aws_vpc_endpoint_connection_accepter" => {
                    bad["resource_changes"][0]["change"]["after"]["service_name"] =
                        "wrong-service".into()
                }
                _ => bad["resource_changes"][2]["change"]["after"]["vpc_id"] = "wrong-vpc".into(),
            }
            let g = semantic::transform(&plan::parse(&bad.to_string()).unwrap());
            assert_eq!(g.nodes.len(), 3, "{kind} case {case}");
            assert_eq!(g.coverage.selected.resources, 3);
        }
    }
}

#[test]
fn helper_ownership_named_filtering_and_private_payloads() {
    for (kind, field, endpoint) in [
        (
            "aws_vpc_endpoint_policy",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
        ),
        (
            "aws_vpc_endpoint_private_dns",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
        ),
        (
            "aws_vpc_endpoint_connection_notification",
            "vpc_endpoint_id",
            "aws_vpc_endpoint",
        ),
        (
            "aws_vpc_endpoint_service_allowed_principal",
            "vpc_endpoint_service_id",
            "aws_vpc_endpoint_service",
        ),
        (
            "aws_vpc_endpoint_service_private_dns_verification",
            "service_id",
            "aws_vpc_endpoint_service",
        ),
    ] {
        let mut v = input(kind, field, endpoint, "id", false);
        v["resource_changes"][1]["change"]["after"]["policy"] = "PRIVATE_PAYLOAD".into();
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(g.nodes[1].member_owner.is_some(), "{kind}");
        let mut filtered = v.clone();
        filtered["resource_changes"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(!verified(&filtered, field));
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
            assert!(!out.contains("PRIVATE_PAYLOAD"));
            assert!(out.contains("data-state-id=\"one\""));
            assert!(out.contains("data-state-id=\"two\""));
        }
    }
    let mut v = input(
        "aws_vpc_endpoint_connection_notification",
        "vpc_endpoint_id",
        "aws_vpc_endpoint",
        "id",
        false,
    );
    v["resource_changes"][1]["change"]["after"]["vpc_endpoint_service_id"] = "ambiguous".into();
    assert!(!verified(&v, "vpc_endpoint_id"));
}
