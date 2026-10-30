use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn v2_new_references_preserve_independent_coverage_filtering_and_named_states() {
    let input = include_str!("../../fixtures/api-gateway-v2-plan.json");
    let raw = plan::parse(input).unwrap();
    let graph = semantic::transform(&raw);
    for address in [
        "aws_apigatewayv2_api.app",
        "aws_service_discovery_service.backend",
        "aws_iam_role.backend",
    ] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert!(node.member_owner.is_none());
    }
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_apigatewayv2_api.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing: Value = serde_json::from_str(input).unwrap();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["address"] != "aws_service_discovery_service.backend");
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: input.into(),
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
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(!output.contains("SYNTHETIC_SECRET_NOT_FOR_OUTPUT"));
    }
}

#[test]
fn v2_service_action_or_unknown_subtype_does_not_claim_lambda_backend() {
    for unknown in [false, true] {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json")).unwrap();
        let integration = value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == "aws_apigatewayv2_integration.lambda")
            .unwrap();
        integration["change"]["after"]["integration_subtype"] = json!("SQS-SendMessage");
        if unknown {
            integration["change"]["after_unknown"]["integration_subtype"] = json!(true);
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        let target = raw
            .nodes
            .iter()
            .position(|n| n.address == "aws_apigatewayv2_integration.lambda")
            .unwrap();
        assert!(
            raw.attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == "integration_uri")
                .all(|r| !r.service_reference_verified)
        );
        assert!(raw.attributes.iter().any(|r| r.target == target
            && r.attribute == "credentials_arn"
            && r.service_reference_verified));
    }
}

#[test]
fn v2_quick_create_roles_and_cloud_map_require_explicit_matching_evidence() {
    for kind in ["quick", "quick_role", "role", "cloud_map"] {
        for case in [
            "valid",
            "unknown",
            "literal",
            "mismatch",
            "deleted",
            "data",
            "deposed",
            "wrong_protocol",
            "wrong_provider",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json"))
                    .unwrap();
            let (address, field, endpoint, resource_type, arn) = match kind {
                "quick" => (
                    "aws_apigatewayv2_api.app",
                    "target",
                    "aws_lambda_function.app",
                    "aws_lambda_function",
                    "arn:aws:lambda:us-east-1:123456789012:function:synthetic",
                ),
                "quick_role" => (
                    "aws_apigatewayv2_api.app",
                    "credentials_arn",
                    "aws_iam_role.backend",
                    "aws_iam_role",
                    "arn:aws:iam::123456789012:role/backend",
                ),
                "role" => (
                    "aws_apigatewayv2_integration.lambda",
                    "credentials_arn",
                    "aws_iam_role.backend",
                    "aws_iam_role",
                    "arn:aws:iam::123456789012:role/backend",
                ),
                _ => (
                    "aws_apigatewayv2_integration.private",
                    "integration_uri",
                    "aws_service_discovery_service.backend",
                    "aws_service_discovery_service",
                    "arn:aws:servicediscovery:us-east-1:123456789012:service/srv-synthetic",
                ),
            };
            if !value["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["address"] == endpoint)
            {
                value["resource_changes"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":resource_type,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"arn":arn}}}));
                value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":resource_type,"provider_config_key":"aws","expressions":{}}));
            }
            for r in value["resource_changes"].as_array_mut().unwrap() {
                if r["address"] == "aws_apigatewayv2_api.app" {
                    r["change"]["after"]["target"] =
                        json!("arn:aws:lambda:us-east-1:123456789012:function:synthetic");
                    if case == "wrong_protocol" {
                        r["change"]["after"]["protocol_type"] = json!("INVALID");
                    }
                }
                if r["address"] == address {
                    r["change"]["after"][field] = json!(arn);
                    if case == "unknown" {
                        r["change"]["after_unknown"][field] = json!(true);
                    }
                    if case == "mismatch" {
                        r["change"]["after"][field] = json!("other");
                    }
                }
                if r["address"] == endpoint {
                    match case {
                        "deleted" => r["change"]["actions"] = json!(["delete"]),
                        "data" => r["mode"] = json!("data"),
                        "deposed" => r["deposed"] = json!("old"),
                        "wrong_provider" => {
                            r["provider_name"] = json!("example.invalid/custom/aws")
                        }
                        _ => {}
                    }
                }
            }
            for r in value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
            {
                if r["address"] == "aws_apigatewayv2_api.app" {
                    r["expressions"]["target"] =
                        json!({"references":["aws_lambda_function.app.arn"]});
                }
                if r["address"] == address {
                    r["expressions"][field] = if case == "literal" {
                        json!({"constant_value":arn})
                    } else {
                        json!({"references":[format!("{endpoint}.arn")]})
                    };
                }
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let target = raw.nodes.iter().position(|n| n.address == address).unwrap();
            assert_eq!(
                raw.attributes.iter().any(|r| r.target == target
                    && r.attribute == field
                    && r.service_reference_verified),
                case == "valid",
                "{kind}: {case}"
            );
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
        }
    }
}

#[test]
fn private_rest_endpoint_sets_require_complete_known_private_configuration() {
    for case in [
        "valid",
        "reordered",
        "partial",
        "duplicate",
        "unknown_id",
        "unknown_types",
        "unknown_block",
        "regional",
        "mixed_types",
        "multiple_blocks",
        "gateway",
        "literal",
        "data",
        "deleted",
        "deposed",
        "wrong_provider",
        "unknown_region",
    ] {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                .unwrap();
        for (name, id) in [("one", "vpce-one"), ("two", "vpce-two")] {
            value["resource_changes"].as_array_mut().unwrap().push(json!({"address":format!("aws_vpc_endpoint.{name}"),"mode":"managed","type":"aws_vpc_endpoint","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"id":id,"vpc_endpoint_type":"Interface"}}}));
            value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":format!("aws_vpc_endpoint.{name}"),"mode":"managed","type":"aws_vpc_endpoint","name":name,"provider_config_key":"aws","expressions":{}}));
        }
        let api = value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == "aws_api_gateway_rest_api.app")
            .unwrap();
        api["change"]["after"]["endpoint_configuration"] =
            json!([{"types":["PRIVATE"],"vpc_endpoint_ids":["vpce-one","vpce-two"]}]);
        let config = &mut api["change"]["after"]["endpoint_configuration"];
        match case {
            "reordered" => config[0]["vpc_endpoint_ids"] = json!(["vpce-two", "vpce-one"]),
            "partial" => {
                config[0]["vpc_endpoint_ids"] = json!(["vpce-one", "vpce-two", "vpce-other"])
            }
            "duplicate" => config[0]["vpc_endpoint_ids"] = json!(["vpce-one", "vpce-one"]),
            "regional" => config[0]["types"] = json!(["REGIONAL"]),
            "mixed_types" => config[0]["types"] = json!(["PRIVATE", "REGIONAL"]),
            "multiple_blocks" => config
                .as_array_mut()
                .unwrap()
                .push(json!({"types":["PRIVATE"],"vpc_endpoint_ids":["vpce-one"]})),
            _ => {}
        }
        match case {
            "unknown_id" => {
                api["change"]["after_unknown"] =
                    json!({"endpoint_configuration":[{"vpc_endpoint_ids":[false,true]}]})
            }
            "unknown_types" => {
                api["change"]["after_unknown"] =
                    json!({"endpoint_configuration":[{"types":[true]}]})
            }
            "unknown_block" => {
                api["change"]["after_unknown"] = json!({"endpoint_configuration":true})
            }
            _ => {}
        }
        let endpoint = value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        match case {
            "gateway" => endpoint["change"]["after"]["vpc_endpoint_type"] = json!("Gateway"),
            "data" => endpoint["mode"] = json!("data"),
            "deleted" => endpoint["change"]["actions"] = json!(["delete"]),
            "deposed" => endpoint["deposed"] = json!("old"),
            "wrong_provider" => endpoint["provider_name"] = json!("example.invalid/custom/aws"),
            _ => {}
        }
        let api = value["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == "aws_api_gateway_rest_api.app")
            .unwrap();
        api["expressions"]["endpoint_configuration"] = if case == "literal" {
            json!([{"vpc_endpoint_ids":{"constant_value":["vpce-one","vpce-two"]}}])
        } else {
            json!([{"vpc_endpoint_ids":{"references":["aws_vpc_endpoint.one.id","aws_vpc_endpoint.two.id"]}}])
        };
        if case == "unknown_region" {
            value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                json!({"references":["var.region"]});
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        let api = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_api_gateway_rest_api.app")
            .unwrap();
        assert!(api.member_owner.is_none());
        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| edge.kind == crate::model::EdgeKind::Containment
                    && graph.nodes[edge.to].address == "aws_api_gateway_rest_api.app")
        );
        assert_eq!(
            api.references
                .iter()
                .any(|r| r.label == "Private REST endpoints" && r.complete),
            matches!(case, "valid" | "reordered"),
            "{case}: {:?}",
            raw.attributes
        );
        assert_eq!(
            graph.coverage.selected.resources,
            value["resource_changes"].as_array().unwrap().len()
        );
        if case == "valid" {
            let selected = crate::view::single(
                &graph,
                &crate::view::Options {
                    focus: Some("aws_api_gateway_rest_api.app".into()),
                    depth: Some(0),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(selected.coverage.selected.resources, 1);
            let mut missing = value.clone();
            missing["resource_changes"].as_array_mut().unwrap().pop();
            for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
                let out = crate::generate_named(
                    &[
                        crate::NamedInput {
                            state: "one".into(),
                            json: value.to_string(),
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
                assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
                assert!(!out.contains("SYNTHETIC_SECRET_NOT_FOR_OUTPUT"));
            }
        }
    }
}

#[test]
fn rest_v2_backend_and_role_require_matching_explicit_context() {
    for role in [false, true] {
        for case in [
            "valid",
            "unknown",
            "literal",
            "mismatch",
            "wrong_type",
            "wrong_method",
            "legacy_link",
            "deleted",
            "data",
            "deposed",
            "application",
            "wrong_provider",
            "unknown_region",
            "unknown_endpoint",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let address = if role {
                "aws_api_gateway_integration.lambda"
            } else {
                "aws_api_gateway_integration.private"
            };
            let field = if role {
                "credentials"
            } else {
                "integration_target"
            };
            let endpoint = if role {
                "aws_iam_role.backend"
            } else {
                "aws_lb.backend"
            };
            let arn = if role {
                "arn:aws:iam::123456789012:role/backend"
            } else {
                "arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/net/synthetic/123"
            };
            if role {
                value["resource_changes"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":"aws_iam_role","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"arn":arn}}}));
                value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":"aws_iam_role","name":"backend","provider_config_key":"aws","expressions":{}}));
            }
            for r in value["resource_changes"].as_array_mut().unwrap() {
                if r["address"] == address {
                    r["change"]["after"][field] = json!(arn);
                    if case == "unknown" {
                        r["change"]["after_unknown"][field] = json!(true);
                    }
                    if case == "mismatch" {
                        r["change"]["after"][field] = json!("other");
                    }
                    if case == "wrong_type" {
                        r["change"]["after"]["type"] = json!("MOCK");
                    }
                    if case == "wrong_method" {
                        r["change"]["after"]["http_method"] = json!("PATCH");
                    }
                }
                if r["address"] == "aws_api_gateway_vpc_link.private" && case != "legacy_link" {
                    r["type"] = json!("aws_apigatewayv2_vpc_link");
                }
                if r["address"] == endpoint {
                    if case == "application" && !role {
                        r["change"]["after"]["load_balancer_type"] = json!("application");
                    }
                    if case == "wrong_provider" {
                        r["provider_name"] = json!("example.invalid/custom/aws");
                    }
                    if case == "unknown_endpoint" {
                        r["change"]["after_unknown"]["arn"] = json!(true);
                    }
                    if case == "deleted" {
                        r["change"]["actions"] = json!(["delete"]);
                    }
                    if case == "data" {
                        r["mode"] = json!("data");
                    }
                    if case == "deposed" {
                        r["deposed"] = json!("old");
                    }
                }
            }
            for r in value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
            {
                if r["address"] == address {
                    r["expressions"][field] = if case == "literal" {
                        json!({"constant_value":arn})
                    } else {
                        json!({"references":[format!("{endpoint}.arn")]})
                    };
                }
            }
            if case != "legacy_link" {
                value = serde_json::from_str(
                    &value
                        .to_string()
                        .replace("aws_api_gateway_vpc_link", "aws_apigatewayv2_vpc_link"),
                )
                .unwrap();
            }
            if case == "unknown_region" {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]});
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let target = raw.nodes.iter().position(|n| n.address == address).unwrap();
            assert_eq!(
                raw.attributes.iter().any(|r| r.target == target
                    && r.attribute == field
                    && r.service_reference_verified),
                matches!(case, "valid" | "application") || (role && case == "legacy_link"),
                "{field}: {case}"
            );
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references.iter().any(|r| r.complete
                    && r.label
                        == if role {
                            "REST integration role"
                        } else {
                            "REST private backend"
                        }),
                matches!(case, "valid" | "application") || (role && case == "legacy_link"),
                "presentation: {field}: {case}"
            );
        }
    }
}

#[test]
fn api_gateway_rejects_incompatible_context_and_partial_indexed_sets() {
    for (family, address, attribute, changed, field, replacement) in [
        (
            "rest",
            "aws_api_gateway_integration.lambda",
            "uri",
            "aws_api_gateway_method.get",
            "rest_api_id",
            json!("other-api"),
        ),
        (
            "rest",
            "aws_api_gateway_integration.lambda",
            "uri",
            "aws_api_gateway_integration.lambda",
            "type",
            json!("HTTP_PROXY"),
        ),
        (
            "rest",
            "aws_api_gateway_stage.live",
            "deployment_id",
            "aws_api_gateway_deployment.release",
            "rest_api_id",
            json!("other-api"),
        ),
        (
            "rest",
            "aws_api_gateway_vpc_link.private",
            "target_arns",
            "aws_lb.backend",
            "load_balancer_type",
            json!("application"),
        ),
        (
            "v2",
            "aws_apigatewayv2_integration.lambda",
            "integration_uri",
            "aws_apigatewayv2_api.app",
            "protocol_type",
            json!("UNKNOWN"),
        ),
        (
            "v2",
            "aws_apigatewayv2_integration.private",
            "integration_uri",
            "aws_lb_listener.backend",
            "load_balancer_arn",
            json!("other-owner"),
        ),
        (
            "v2",
            "aws_apigatewayv2_stage.live",
            "deployment_id",
            "aws_apigatewayv2_stage.live",
            "auto_deploy",
            json!(true),
        ),
        (
            "v2",
            "aws_apigatewayv2_authorizer.request",
            "authorizer_uri",
            "aws_apigatewayv2_authorizer.request",
            "authorizer_type",
            json!("JWT"),
        ),
        (
            "v2",
            "aws_apigatewayv2_vpc_link.private",
            "subnet_ids",
            "aws_apigatewayv2_vpc_link.private",
            "subnet_ids",
            json!(["subnet-synthetic-0", "subnet-synthetic-1", "unreferenced"]),
        ),
    ] {
        let input = if family == "rest" {
            include_str!("../../fixtures/api-gateway-rest-plan.json")
        } else {
            include_str!("../../fixtures/api-gateway-v2-plan.json")
        };
        let mut value: Value = serde_json::from_str(input).unwrap();
        let resource = value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == changed)
            .unwrap();
        resource["change"]["after"][field] = replacement;
        let raw = plan::parse(&value.to_string()).unwrap();
        let target = raw.nodes.iter().position(|n| n.address == address).unwrap();
        assert!(
            raw.attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == attribute)
                .all(|r| !r.service_reference_verified),
            "{address}: {field}"
        );
    }
    let mut value: Value =
        serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json")).unwrap();
    let link = value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == "aws_apigatewayv2_vpc_link.private")
        .unwrap();
    link["expressions"]["subnet_ids"] = json!({"references":["aws_subnet.private.id"]});
    let raw = plan::parse(&value.to_string()).unwrap();
    assert!(
        raw.attributes
            .iter()
            .filter(|r| r.attribute == "subnet_ids")
            .all(|r| !r.service_reference_verified)
    );
}

#[test]
fn api_gateway_contexts_lists_and_deployments_are_independently_verified() {
    for (input, checks) in [
        (
            include_str!("../../fixtures/api-gateway-rest-plan.json"),
            vec![
                (
                    "aws_api_gateway_integration.lambda",
                    "uri",
                    "REST Lambda backend",
                ),
                (
                    "aws_api_gateway_integration.private",
                    "connection_id",
                    "REST VPC link",
                ),
                (
                    "aws_api_gateway_authorizer.request",
                    "authorizer_uri",
                    "REST Lambda authorizer",
                ),
                (
                    "aws_api_gateway_stage.live",
                    "deployment_id",
                    "REST deployment",
                ),
                (
                    "aws_api_gateway_vpc_link.private",
                    "target_arns",
                    "REST NLB targets",
                ),
            ],
        ),
        (
            include_str!("../../fixtures/api-gateway-v2-plan.json"),
            vec![
                (
                    "aws_apigatewayv2_integration.private",
                    "integration_uri",
                    "V2 integration backend",
                ),
                (
                    "aws_apigatewayv2_authorizer.request",
                    "authorizer_uri",
                    "V2 Lambda authorizer",
                ),
                (
                    "aws_apigatewayv2_stage.live",
                    "deployment_id",
                    "V2 explicit deployment",
                ),
                (
                    "aws_apigatewayv2_vpc_link.private",
                    "subnet_ids",
                    "V2 link subnets",
                ),
                (
                    "aws_apigatewayv2_vpc_link.private",
                    "security_group_ids",
                    "V2 link security groups",
                ),
            ],
        ),
    ] {
        for (address, attribute, label) in checks {
            for unknown in [false, true] {
                let mut value: Value = serde_json::from_str(input).unwrap();
                if unknown {
                    let change = value["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["address"] == address)
                        .unwrap();
                    change["change"]["after_unknown"][attribute] = json!(true);
                }
                let raw = plan::parse(&value.to_string()).unwrap();
                let graph = semantic::transform(&raw);
                let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
                assert_eq!(
                    node.references
                        .iter()
                        .any(|r| r.label == label && r.complete),
                    !unknown,
                    "{address}: {attribute}: {:?}",
                    raw.attributes
                );
            }
        }
    }
}

#[test]
fn api_gateway_backends_require_explicit_current_matching_evidence() {
    for (input, address, attribute, label) in [
        (
            include_str!("../../fixtures/api-gateway-rest-plan.json"),
            "aws_api_gateway_integration.lambda",
            "uri",
            "REST Lambda backend",
        ),
        (
            include_str!("../../fixtures/api-gateway-v2-plan.json"),
            "aws_apigatewayv2_integration.lambda",
            "integration_uri",
            "V2 integration backend",
        ),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "mismatch",
            "wrong_provider",
            "deleted",
            "deposed",
            "data",
            "unknown_region",
            "moved_imported",
            "wrong_type",
        ] {
            let mut value: Value = serde_json::from_str(input).unwrap();
            let target = value["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .position(|r| r["address"] == address)
                .unwrap();
            let source = value["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .position(|r| r["address"] == "aws_lambda_function.app")
                .unwrap();
            match case {
                "moved_imported" => {
                    value["resource_changes"][target]["previous_address"] =
                        json!("aws_api_gateway_integration.old");
                    value["resource_changes"][target]["change"]["importing"] =
                        json!({"id":"SYNTHETIC_IMPORT_NOT_FOR_OUTPUT"});
                }
                "wrong_type" => value["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "literal" => {
                    let resource = value["configuration"]["root_module"]["resources"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["address"] == address)
                        .unwrap();
                    resource["expressions"][attribute] =
                        json!({"constant_value":"SYNTHETIC_NOT_FOR_OUTPUT"});
                }
                "unknown" => {
                    value["resource_changes"][target]["change"]["after_unknown"][attribute] =
                        json!(true)
                }
                "mismatch" => {
                    value["resource_changes"][target]["change"]["after"][attribute] =
                        json!("SYNTHETIC_NOT_FOR_OUTPUT")
                }
                "wrong_provider" => {
                    value["resource_changes"][source]["provider_name"] =
                        json!("example.invalid/custom/aws")
                }
                "deleted" => {
                    let change = &mut value["resource_changes"][source]["change"];
                    change["before"] = change["after"].take();
                    change["actions"] = json!(["delete"]);
                }
                "deposed" => value["resource_changes"][source]["deposed"] = json!("old"),
                "data" => value["resource_changes"][source]["mode"] = json!("data"),
                "unknown_region" => {
                    value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                _ => {}
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.label == label && r.complete),
                matches!(case, "valid" | "moved_imported"),
                "{address}: {case}"
            );
        }
    }
}
