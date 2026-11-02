use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn nested(path: &str, leaf: Value) -> Value {
    let fields: Vec<_> = path.split('.').collect();
    let mut value = leaf;
    for (index, field) in fields.iter().enumerate().rev() {
        value = json!({*field:value});
        if index != 0 {
            value = json!([value]);
        }
    }
    value
}

fn fixture(kind: &str, path: &str, target: &str, field: &str, collection: bool) -> Value {
    let identifier = if target == "aws_kms_key" {
        "arn:aws:kms:us-west-2:123456789012:key/test"
    } else {
        "fixture-id"
    };
    let source = format!("{target}.target");
    let address = format!("{kind}.test");
    let mut after = nested(
        path,
        if collection {
            json!([identifier])
        } else {
            json!(identifier)
        },
    );
    after["permission_type"] = json!("CUSTOMER_MANAGED");
    after["account_access_type"] = json!("ORGANIZATION");
    json!({"format_version":"1.2", "resource_changes":[
        {"address":source,"type":target,"mode":"managed","change":{"actions":["no-op"],"after":{field:identifier,"vpc_endpoint_type":"Interface","service_name":"com.amazonaws.us-west-2.grafana-workspace"}}},
        {"address":address,"type":kind,"mode":"managed","change":{"actions":["create"],"after":after}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}}},"root_module":{"resources":[
        {"address":source,"provider_config_key":"aws"},
        {"address":address,"provider_config_key":"aws","expressions":nested(path,json!({"references":[format!("{source}.{field}")]}))}
    ]}}})
}

#[test]
fn grafana_workspace_references_reject_unproven_endpoints() {
    for (path, target, field, collection) in [
        ("role_arn", "aws_iam_role", "arn", false),
        ("kms_key_id", "aws_kms_key", "arn", false),
        (
            "organizational_units",
            "aws_organizations_organizational_unit",
            "id",
            true,
        ),
        (
            "network_access_control.prefix_list_ids",
            "aws_ec2_managed_prefix_list",
            "id",
            true,
        ),
        (
            "network_access_control.vpce_ids",
            "aws_vpc_endpoint",
            "id",
            true,
        ),
        ("vpc_configuration.subnet_ids", "aws_subnet", "id", true),
        (
            "vpc_configuration.security_group_ids",
            "aws_security_group",
            "id",
            true,
        ),
    ] {
        for case in 0..10 {
            let mut input = fixture("aws_grafana_workspace", path, target, field, collection);
            match case {
                0 => {}
                1 => input["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
                2 => {
                    input["resource_changes"][0]["provider_name"] =
                        json!("registry.terraform.io/other/aws")
                }
                3 => input["resource_changes"][0]["mode"] = json!("data"),
                4 => input["resource_changes"][0]["deposed"] = json!("old"),
                5 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => input["resource_changes"][0]["change"]["after"][field] = json!("mismatch"),
                7 => input["resource_changes"][0]["change"]["after_unknown"] = json!({field:true}),
                8 => {
                    input["resource_changes"][1]["change"]["after_unknown"] =
                        nested(path, json!(true))
                }
                _ => {
                    input["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
                }
            }
            let parsed = plan::parse(&input.to_string()).unwrap();
            let graph = semantic::transform(&parsed);
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_grafana_workspace.test")
                .unwrap();
            assert_eq!(node.placement, Placement::Spatial);
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.attribute == path && r.label != "References" && r.complete),
                case == 0,
                "{path} case {case}"
            );
            assert_eq!(graph.coverage.selected.resources, 2);
        }
    }
}

#[test]
fn grafana_access_endpoints_require_grafana_interface_service() {
    for (key, value) in [
        ("vpc_endpoint_type", "Gateway"),
        ("service_name", "com.amazonaws.us-west-2.s3"),
    ] {
        let mut input = fixture(
            "aws_grafana_workspace",
            "network_access_control.vpce_ids",
            "aws_vpc_endpoint",
            "id",
            true,
        );
        input["resource_changes"][0]["change"]["after"][key] = json!(value);
        let parsed = plan::parse(&input.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .filter(|r| r.attribute == "network_access_control.vpce_ids")
                .all(|r| !r.service_reference_verified)
        );
    }
}

#[test]
fn grafana_network_requires_complete_common_vpc_without_subnet_ownership() {
    for case in 0..4 {
        let text = include_str!("../../fixtures/msk-plan.json")
            .replace("aws_msk_serverless_cluster", "aws_grafana_workspace")
            .replace("vpc_config", "vpc_configuration");
        let mut input: Value = serde_json::from_str(&text).unwrap();
        match case {
            0 => {}
            1 => {
                input["resource_changes"][11]["change"]["after"]["vpc_configuration"][0]["subnet_ids"] =
                    json!(["subnet-a", "missing"])
            }
            2 => {
                input["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"vpc_configuration":[{"security_group_ids":true}]})
            }
            _ => input["resource_changes"][5]["change"]["after"]["vpc_id"] = json!("vpc-client"),
        }
        let parsed = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        let parents: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| {
                e.kind == EdgeKind::Containment
                    && graph.nodes[e.to].address == "aws_grafana_workspace.serverless"
            })
            .map(|e| graph.nodes[e.from].address.as_str())
            .collect();
        assert_eq!(
            parents,
            if case == 0 {
                vec!["aws_vpc.cluster"]
            } else {
                vec![]
            },
            "case {case}"
        );
    }
}

#[test]
fn grafana_name_references_require_account_qualification() {
    for (path, kind) in [
        ("organization_role_name", "aws_iam_role"),
        ("stack_set_name", "aws_cloudformation_stack_set"),
    ] {
        let input = fixture("aws_grafana_workspace", path, kind, "name", false);
        let mut parsed = plan::parse(&input.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .all(|r| !r.service_reference_verified)
        );
        for node in &mut parsed.nodes {
            node.provider_configuration.as_mut().unwrap().account =
                AwsAccountScope::new("123456789012");
        }
        let evidence = parsed
            .nodes
            .iter()
            .map(|node| {
                let n = input["resource_changes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["address"].as_str() == Some(&node.address))
                    .unwrap();
                (&n["change"]["after"], &n["change"]["after_unknown"])
            })
            .collect::<Vec<_>>();
        super::annotate(&parsed.nodes, &mut parsed.attributes, &evidence);
        assert!(
            parsed
                .attributes
                .iter()
                .any(|r| r.attribute == path && r.service_reference_verified),
            "{path}: {:?}",
            parsed.attributes
        );
    }
}

#[test]
fn prometheus_workspace_settings_preserve_identity_and_require_log_suffix_provenance() {
    for case in 0..7 {
        let mut input: Value =
            serde_json::from_str(include_str!("../../fixtures/prometheus-plan.json")).unwrap();
        match case {
            0 => {}
            1 => {
                input["resource_changes"][8]["change"]["after"]["destination"][0]["cloudwatch_logs"]
                    [0]["log_group_arn"] =
                    input["resource_changes"][1]["change"]["after"]["arn"].clone()
            }
            2 => input["resource_changes"][1]["change"]["after_unknown"] = json!({"arn":true}),
            3 => input["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            4 => {
                input["configuration"]["root_module"]["resources"][8]["expressions"]["destination"] =
                    json!([])
            }
            5 => {
                input["resource_changes"][8]["change"]["after_unknown"] =
                    json!({"destination":true})
            }
            _ => input["resource_changes"][1]["type"] = json!("aws_sqs_queue"),
        }
        let parsed = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.coverage.selected.resources, 9);
        let query = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_prometheus_query_logging_configuration")
            .unwrap();
        assert_eq!(
            query
                .references
                .iter()
                .any(|r| r.label == "Query log groups" && r.complete),
            case == 0,
            "case {case}"
        );
        if case == 0 {
            let workspace = graph
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_prometheus_workspace")
                .unwrap();
            assert_eq!(workspace.placement, Placement::Spatial);
            assert!(
                workspace
                    .references
                    .iter()
                    .any(|r| r.label == "Workspace log group" && r.complete)
            );
            for helper in graph.nodes.iter().filter(|n| {
                n.resource_type.starts_with("aws_prometheus_")
                    && n.resource_type != "aws_prometheus_workspace"
            }) {
                assert_eq!(helper.placement, Placement::Logical);
                assert!(
                    helper
                        .references
                        .iter()
                        .any(|r| r.label == "Prometheus workspace owner" && r.complete)
                );
            }
        }
    }
}

#[test]
fn prometheus_query_log_destinations_require_the_complete_collection() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/prometheus-plan.json")).unwrap();
    let mut log = input["resource_changes"][1].clone();
    log["address"] = json!("aws_cloudwatch_log_group.other");
    log["change"]["after"]["arn"] = json!("arn:aws:logs:us-west-2:123456789012:log-group:other");
    input["resource_changes"].as_array_mut().unwrap().push(log);
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_cloudwatch_log_group.other","provider_config_key":"aws"}));
    input["resource_changes"][8]["change"]["after"]["destination"].as_array_mut().unwrap().push(json!({"cloudwatch_logs":[{"log_group_arn":"arn:aws:logs:us-west-2:123456789012:log-group:other:*"}]}));
    input["configuration"]["root_module"]["resources"][8]["expressions"]["destination"].as_array_mut().unwrap().push(json!({"cloudwatch_logs":[{"log_group_arn":{"references":["aws_cloudwatch_log_group.other.arn"]}}]}));
    for valid in [true, false] {
        if !valid {
            input["resource_changes"][8]["change"]["after_unknown"] =
                json!({"destination":[{}, {"cloudwatch_logs":true}]});
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        let query = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_prometheus_query_logging_configuration")
            .unwrap();
        assert_eq!(
            query
                .references
                .iter()
                .any(|r| r.label == "Query log groups" && r.complete && r.targets.len() == 2),
            valid
        );
    }
}

#[test]
fn prometheus_helper_owner_proof_is_typed_current_and_known() {
    for kind in [
        "aws_prometheus_alert_manager_definition",
        "aws_prometheus_anomaly_detector",
        "aws_prometheus_resource_policy",
        "aws_prometheus_rule_group_namespace",
        "aws_prometheus_workspace_configuration",
        "aws_prometheus_query_logging_configuration",
    ] {
        for case in 0..5 {
            let mut input = fixture(
                kind,
                "workspace_id",
                "aws_prometheus_workspace",
                "id",
                false,
            );
            match case {
                0 => {}
                1 => input["resource_changes"][0]["type"] = json!("aws_grafana_workspace"),
                2 => input["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
                3 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                _ => {
                    input["resource_changes"][0]["provider_name"] =
                        json!("registry.terraform.io/other/aws")
                }
            }
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            let node = graph
                .nodes
                .iter()
                .find(|n| n.resource_type == kind)
                .unwrap();
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.label == "Prometheus workspace owner" && r.complete),
                case == 0,
                "{kind} case {case}"
            );
        }
    }
}

#[test]
fn grafana_helpers_keep_actions_coverage_and_workspace_qualified_tokens() {
    for case in 0..7 {
        let mut input: Value =
            serde_json::from_str(include_str!("../../fixtures/grafana-owners-plan.json")).unwrap();
        match case {
            0 => {}
            1 => {
                input["resource_changes"][10]["change"]["after"]["workspace_id"] =
                    json!("other-workspace")
            }
            2 => {
                input["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"workspace_id":true})
            }
            3 => input["resource_changes"][10]["change"]["actions"] = json!(["delete"]),
            4 => input["resource_changes"][10]["mode"] = json!("data"),
            5 => {
                input["resource_changes"][10]["change"]["after_unknown"] =
                    json!({"service_account_id":true})
            }
            _ => input["configuration"]["root_module"]["resources"][10]["expressions"] = json!({}),
        }
        let parsed = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.coverage.selected.resources, 12);
        let token = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_grafana_workspace_service_account_token")
            .unwrap();
        assert_eq!(token.placement, Placement::Logical);
        assert_eq!(
            token
                .references
                .iter()
                .any(|r| r.attribute == "service_account_id"
                    && r.complete
                    && r.label == "Grafana service account owner"),
            case == 0,
            "case {case}"
        );
        if case == 0 {
            for node in graph.nodes.iter().filter(|n| {
                n.resource_type.starts_with("aws_grafana_")
                    && n.resource_type != "aws_grafana_workspace"
            }) {
                assert_eq!(node.action, Action::Create);
                assert!(
                    node.references
                        .iter()
                        .any(|r| r.attribute == "workspace_id" && r.complete)
                );
            }
        }
    }
}

#[test]
fn grafana_helper_credentials_are_not_emitted() {
    let input = include_str!("../../fixtures/grafana-owners-plan.json");
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            input,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_TOKEN"));
        assert!(!output.content.contains("PRIVATE_SAML"));
    }
}
