use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn v2_stage_logging_and_certificates_require_protocol_and_explicit_evidence() {
    for certificate in [false, true] {
        for case in [
            "valid",
            "http",
            "unknown_protocol",
            "literal",
            "unknown",
            "extra_block",
            "wrong_type",
            "unknown_endpoint",
            "deleted",
            "data",
            "deposed",
            "wrong_provider",
            "unknown_region",
            "automatic",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json"))
                    .unwrap();
            let (kind, field, output, identity, label) = if certificate {
                (
                    "aws_api_gateway_client_certificate",
                    "client_certificate_id",
                    "id",
                    "certificate",
                    "V2 client certificate",
                )
            } else {
                (
                    "aws_cloudwatch_log_group",
                    "access_log_settings.destination_arn",
                    "arn",
                    "arn:aws:logs:us-east-1:123456789012:log-group:access",
                    "V2 access logs",
                )
            };
            let endpoint = format!("{kind}.configuration");
            let address = "aws_apigatewayv2_stage.live";
            let mut resource = json!({"address":endpoint,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{output:identity}}});
            match case {
                "wrong_type" => resource["type"] = json!("aws_kinesis_firehose_delivery_stream"),
                "unknown_endpoint" => resource["change"]["after_unknown"][output] = json!(true),
                "deleted" => resource["change"]["actions"] = json!(["delete"]),
                "data" => resource["mode"] = json!("data"),
                "deposed" => resource["deposed"] = json!("old"),
                "wrong_provider" => resource["provider_name"] = json!("example.invalid/custom/aws"),
                _ => {}
            }
            value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(resource);
            value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":kind,"name":"configuration","provider_config_key":"aws","expressions":{}}));
            for r in value["resource_changes"].as_array_mut().unwrap() {
                if r["address"] == "aws_apigatewayv2_api.app" {
                    r["change"]["after"]["protocol_type"] =
                        json!(if case == "http" { "HTTP" } else { "WEBSOCKET" });
                    if case == "unknown_protocol" {
                        r["change"]["after_unknown"]["protocol_type"] = json!(true);
                    }
                }
                if r["address"] == address {
                    if certificate {
                        r["change"]["after"][field] = json!(identity);
                        if case == "unknown" {
                            r["change"]["after_unknown"][field] = json!(true);
                        }
                    } else {
                        r["change"]["after"]["access_log_settings"] =
                            json!([{"destination_arn":identity}]);
                        if case == "unknown" {
                            r["change"]["after_unknown"]["access_log_settings"] =
                                json!([{"destination_arn":true}]);
                        }
                        if case == "extra_block" {
                            r["change"]["after"]["access_log_settings"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!({"destination_arn":identity}));
                        }
                    }
                    if case == "automatic" {
                        r["change"]["after"]["auto_deploy"] = json!(true);
                    }
                }
            }
            let expressions = &mut value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap()["expressions"];
            let expression = if case == "literal" {
                json!({"constant_value":identity})
            } else {
                json!({"references":[format!("{endpoint}.{output}")]})
            };
            if certificate {
                expressions[field] = expression;
            } else {
                expressions["access_log_settings"] = json!([{"destination_arn":expression}]);
            }
            if case == "unknown_region" {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]});
            }
            let expected = matches!(case, "valid" | "automatic")
                || (case == "http" && !certificate)
                || (case == "extra_block" && certificate);
            let raw = plan::parse(&value.to_string()).unwrap();
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{field} {case}"
            );
            let graph = semantic::transform(&raw);
            if case == "valid" {
                let selected = crate::view::single(
                    &graph,
                    &crate::view::Options {
                        focus: Some(address.into()),
                        depth: Some(0),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(selected.coverage.selected.resources, 1);
                let mut missing = value.clone();
                missing["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|r| r["address"] != endpoint);
                let missing_raw = plan::parse(&missing.to_string()).unwrap();
                assert!(!missing_raw.attributes.iter().any(
                    |r| missing_raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified
                ));
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
                    &crate::Options::default(),
                )
                .unwrap()
                .content;
                assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
            }
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
            assert!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == endpoint)
                    .unwrap()
                    .member_owner
                    .is_none()
            );
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == address)
                    .unwrap()
                    .references
                    .iter()
                    .any(|r| r.complete && r.label == label),
                expected,
                "presentation {field} {case}"
            );
        }
    }
}

#[test]
fn rest_stage_configuration_requires_current_explicit_matching_evidence() {
    for (field, endpoint_type, output, identity, label) in [
        (
            "client_certificate_id",
            "aws_api_gateway_client_certificate",
            "id",
            "certificate",
            "REST client certificate",
        ),
        (
            "documentation_version",
            "aws_api_gateway_documentation_version",
            "version",
            "v1",
            "REST documentation version",
        ),
        (
            "canary_settings.deployment_id",
            "aws_api_gateway_deployment",
            "id",
            "canary",
            "REST canary deployment",
        ),
        (
            "access_log_settings.destination_arn",
            "aws_cloudwatch_log_group",
            "arn",
            "arn:aws:logs:us-east-1:123456789012:log-group:access",
            "REST access logs",
        ),
        (
            "access_log_settings.destination_arn",
            "aws_kinesis_firehose_delivery_stream",
            "arn",
            "arn:aws:firehose:us-east-1:123456789012:deliverystream/amazon-apigateway-access",
            "REST access logs",
        ),
        (
            "cloudwatch_role_arn",
            "aws_iam_role",
            "arn",
            "arn:aws:iam::123456789012:role/logging",
            "API Gateway logging role",
        ),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "mismatch",
            "deleted",
            "data",
            "deposed",
            "wrong_provider",
            "wrong_parent",
            "extra_block",
            "bad_stream_name",
            "unknown_endpoint",
            "wrong_type",
            "unknown_region",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let endpoint = format!("{endpoint_type}.configuration");
            let stage = "aws_api_gateway_stage.live";
            let account = "aws_api_gateway_account.configuration";
            let address = if field == "cloudwatch_role_arn" {
                account
            } else {
                stage
            };
            let api_value = value["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["address"] == stage)
                .unwrap()["change"]["after"]["rest_api_id"]
                .clone();
            let api_expression = value["configuration"]["root_module"]["resources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["address"] == stage)
                .unwrap()["expressions"]["rest_api_id"]
                .clone();
            let mut after = json!({"rest_api_id":api_value,"name":"amazon-apigateway-access"});
            after[output] = json!(identity);
            if case == "wrong_parent" {
                after["rest_api_id"] = json!("other");
            }
            if case == "bad_stream_name" {
                after["name"] = json!("invalid");
            }
            let mut resource = json!({"address":endpoint,"mode":"managed","type":endpoint_type,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":after}});
            match case {
                "unknown_endpoint" => resource["change"]["after_unknown"][output] = json!(true),
                "wrong_type" => resource["type"] = json!("aws_s3_bucket"),
                "deleted" => resource["change"]["actions"] = json!(["delete"]),
                "data" => resource["mode"] = json!("data"),
                "deposed" => resource["deposed"] = json!("old"),
                "wrong_provider" => resource["provider_name"] = json!("example.invalid/custom/aws"),
                _ => {}
            }
            value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(resource);
            value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":endpoint,"mode":"managed","type":endpoint_type,"name":"configuration","provider_config_key":"aws","expressions":{"rest_api_id":api_expression}}));
            if address == account {
                value["resource_changes"].as_array_mut().unwrap().push(json!({"address":account,"mode":"managed","type":"aws_api_gateway_account","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{}}}));
                value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":account,"mode":"managed","type":"aws_api_gateway_account","name":"logging","provider_config_key":"aws","expressions":{}}));
            }
            let change = &mut value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap()["change"];
            let actual = if case == "mismatch" {
                "other"
            } else {
                identity
            };
            if let Some((block, key)) = field.split_once('.') {
                change["after"][block] = json!([{key:actual}]);
                if case == "unknown" {
                    change["after_unknown"][block] = json!([{key:true}]);
                }
                if case == "extra_block" {
                    change["after"][block]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({key:actual}));
                }
            } else {
                change["after"][field] = json!(actual);
                if case == "unknown" {
                    change["after_unknown"][field] = json!(true);
                }
            }
            let expressions = &mut value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap()["expressions"];
            let expression = if case == "literal" {
                json!({"constant_value":identity})
            } else {
                json!({"references":[format!("{endpoint}.{output}")]})
            };
            if let Some((block, key)) = field.split_once('.') {
                expressions[block] = json!([{key:expression}]);
            } else {
                expressions[field] = expression;
            }
            let expected = case == "valid"
                || (case == "wrong_parent"
                    && !matches!(
                        field,
                        "documentation_version" | "canary_settings.deployment_id"
                    ))
                || (case == "extra_block" && !field.contains('.'))
                || (case == "bad_stream_name"
                    && endpoint_type != "aws_kinesis_firehose_delivery_stream");
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
                expected,
                "{field} {endpoint_type} {case}"
            );
            let graph = semantic::transform(&raw);
            if case == "valid" {
                assert!(
                    graph
                        .nodes
                        .iter()
                        .find(|n| n.address == endpoint)
                        .unwrap()
                        .member_owner
                        .is_none()
                );
                let selected = crate::view::single(
                    &graph,
                    &crate::view::Options {
                        focus: Some(address.into()),
                        depth: Some(0),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(selected.coverage.selected.resources, 1);
                let mut missing = value.clone();
                missing["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|r| r["address"] != endpoint);
                let missing_raw = plan::parse(&missing.to_string()).unwrap();
                assert!(!missing_raw.attributes.iter().any(
                    |r| missing_raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified
                ));
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
                    &crate::Options::default(),
                )
                .unwrap()
                .content;
                assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
            }
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == address)
                    .unwrap()
                    .references
                    .iter()
                    .any(|r| r.label == label && r.complete),
                expected,
                "presentation {field} {case}"
            );
        }
    }
}

#[test]
fn v2_route_qualifiers_and_model_map_are_conservative() {
    for case in [
        "jwt_http",
        "jwt_websocket",
        "none",
        "wrong_authorizer",
        "partial_map",
        "unknown_entry",
        "duplicate_model",
        "http_models",
        "target_suffix",
    ] {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json")).unwrap();
        for r in value["resource_changes"].as_array_mut().unwrap() {
            if r["address"] == "aws_apigatewayv2_route.http" {
                if case.starts_with("jwt_") {
                    r["change"]["after"]["authorization_type"] = json!("JWT");
                }
                if case == "none" {
                    r["change"]["after"]["authorization_type"] = json!("NONE");
                }
                if case == "target_suffix" {
                    r["change"]["after"]["target"] = json!("integrations/integration-lambda/extra");
                }
            }
            if r["address"] == "aws_apigatewayv2_authorizer.request"
                && (case.starts_with("jwt_") || case == "wrong_authorizer")
            {
                r["change"]["after"]["authorizer_type"] = json!("JWT");
            }
            if r["address"] == "aws_apigatewayv2_api.app" && case == "jwt_websocket" {
                r["change"]["after"]["protocol_type"] = json!("WEBSOCKET");
            }
            if r["address"] == "aws_apigatewayv2_api.socket" && case == "http_models" {
                r["change"]["after"]["protocol_type"] = json!("HTTP");
            }
            if r["address"] == "aws_apigatewayv2_model.two" && case == "duplicate_model" {
                r["change"]["after"]["name"] = json!("Modelone");
            }
            if r["address"] == "aws_apigatewayv2_route.socket" {
                if case == "partial_map" {
                    r["change"]["after"]["request_models"]["extra"] = json!("Unreferenced");
                }
                if case == "unknown_entry" {
                    r["change"]["after_unknown"]["request_models"] = json!({"one":true});
                }
                if case == "duplicate_model" {
                    r["change"]["after"]["request_models"]["two"] = json!("Modelone");
                }
            }
        }
        let (address, field) = if matches!(
            case,
            "partial_map" | "unknown_entry" | "duplicate_model" | "http_models"
        ) {
            ("aws_apigatewayv2_route.socket", "request_models")
        } else {
            (
                "aws_apigatewayv2_route.http",
                if case == "target_suffix" {
                    "target"
                } else {
                    "authorizer_id"
                },
            )
        };
        let raw = plan::parse(&value.to_string()).unwrap();
        let target = raw.nodes.iter().position(|n| n.address == address).unwrap();
        assert_eq!(
            raw.attributes.iter().any(|r| r.target == target
                && r.attribute == field
                && r.service_reference_verified),
            case == "jwt_http",
            "{case}"
        );
    }
}

#[test]
fn v2_routes_and_authorizer_roles_require_complete_qualified_endpoints() {
    for (address, field, endpoint) in [
        (
            "aws_apigatewayv2_authorizer.request",
            "authorizer_credentials_arn",
            "aws_iam_role.backend",
        ),
        (
            "aws_apigatewayv2_route.http",
            "target",
            "aws_apigatewayv2_integration.lambda",
        ),
        (
            "aws_apigatewayv2_route.http",
            "authorizer_id",
            "aws_apigatewayv2_authorizer.request",
        ),
        (
            "aws_apigatewayv2_route.socket",
            "request_models",
            "aws_apigatewayv2_model.two",
        ),
    ] {
        for case in [
            "valid",
            "unknown",
            "literal",
            "deleted",
            "data",
            "deposed",
            "wrong_parent",
            "wrong_provider",
            "mismatch",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-v2-plan.json"))
                    .unwrap();
            for r in value["resource_changes"].as_array_mut().unwrap() {
                if r["address"] == address {
                    if case == "unknown" {
                        r["change"]["after_unknown"][field] = json!(true);
                    }
                    if case == "mismatch" {
                        r["change"]["after"][field] = if field == "request_models" {
                            json!({"one":"Modelone","two":"other"})
                        } else {
                            json!("other")
                        };
                    }
                    if case == "wrong_parent" && field == "authorizer_credentials_arn" {
                        r["change"]["after"]["authorizer_type"] = json!("JWT");
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
                        "wrong_parent" if field != "authorizer_credentials_arn" => {
                            r["change"]["after"]["api_id"] = json!("other")
                        }
                        _ => {}
                    }
                }
            }
            if case == "literal" {
                value["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == address)
                    .unwrap()["expressions"][field] = json!({"constant_value":"opaque"});
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let target = raw.nodes.iter().position(|n| n.address == address).unwrap();
            assert_eq!(
                raw.attributes.iter().any(|r| r.target == target
                    && r.attribute == field
                    && r.service_reference_verified),
                case == "valid",
                "{field}: {case}"
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

#[test]
fn rest_helpers_require_explicit_matching_api_context() {
    for kind in [
        "aws_api_gateway_documentation_part",
        "aws_api_gateway_documentation_version",
        "aws_api_gateway_gateway_response",
        "aws_api_gateway_integration_response",
        "aws_api_gateway_method_response",
        "aws_api_gateway_method_settings",
        "aws_api_gateway_model",
        "aws_api_gateway_request_validator",
        "aws_api_gateway_rest_api_policy",
        "aws_api_gateway_rest_api_put",
    ] {
        for case in ["valid", "literal", "mismatch", "unknown", "deleted", "data"] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let address = format!("{kind}.api_context_test");
            let mut helper = json!({"address":address,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"rest_api_id":"rest-synthetic"}}});
            match case {
                "mismatch" => helper["change"]["after"]["rest_api_id"] = json!("other"),
                "unknown" => helper["change"]["after_unknown"]["rest_api_id"] = json!(true),
                "deleted" => helper["change"]["actions"] = json!(["delete"]),
                "data" => helper["mode"] = json!("data"),
                _ => {}
            }
            value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(helper);
            let expression = if case == "literal" {
                json!({"constant_value":"rest-synthetic"})
            } else {
                json!({"references":["aws_api_gateway_rest_api.app.id"]})
            };
            value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"mode":"managed","type":kind,"name":"helper","provider_config_key":"aws","expressions":{"rest_api_id":expression}}));
            let raw = plan::parse(&value.to_string()).unwrap();
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == "rest_api_id"
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
fn rest_helper_qualifiers_reject_cross_api_partial_models_and_parent_cycles() {
    for (address, field, endpoint) in [
        (
            "aws_api_gateway_resource.path",
            "parent_id",
            "aws_api_gateway_rest_api.app",
        ),
        (
            "aws_api_gateway_method_settings.all",
            "stage_name",
            "aws_api_gateway_stage.live",
        ),
        (
            "aws_api_gateway_method_response.ok",
            "response_models",
            "aws_api_gateway_model.response",
        ),
        (
            "aws_api_gateway_integration_response.ok",
            "status_code",
            "aws_api_gateway_method_response.ok",
        ),
        (
            "aws_api_gateway_method_response.ok",
            "http_method",
            "aws_api_gateway_method.get",
        ),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "other_api",
            "data",
            "deposed",
            "missing",
            "partial",
            "cycle",
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let resources = value["resource_changes"].as_array_mut().unwrap();
            let target = resources
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            if case == "unknown" {
                target["change"]["after_unknown"][field] = json!(true);
            }
            if case == "partial" && field == "response_models" {
                target["change"]["after"][field]["text/plain"] = json!("Unreferenced");
            }
            if case == "cycle" && field == "parent_id" {
                target["change"]["after"][field] = json!("resource-synthetic");
            }
            let source = resources
                .iter_mut()
                .find(|r| r["address"] == endpoint)
                .unwrap();
            match case {
                "other_api" => {
                    if field == "parent_id" {
                        source["change"]["after"]["id"] = json!("other");
                    } else {
                        source["change"]["after"]["rest_api_id"] = json!("other");
                    }
                }
                "data" => source["mode"] = json!("data"),
                "deposed" => source["deposed"] = json!("old"),
                _ => {}
            }
            if case == "missing" {
                resources.retain(|r| r["address"] != endpoint);
            }
            let configuration = value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            if case == "literal" {
                configuration["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            if case == "cycle" && field == "parent_id" {
                configuration["expressions"][field] =
                    json!({"references":["aws_api_gateway_resource.path.id"]});
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let expected = case == "valid"
                || (case == "partial" && field != "response_models")
                || (case == "cycle" && field != "parent_id");
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field}: {case}"
            );
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
            if case == "valid" {
                let selected = crate::view::single(
                    &graph,
                    &crate::view::Options {
                        focus: Some(address.into()),
                        depth: Some(0),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(selected.coverage.selected.resources, 1);
                assert!(
                    graph
                        .nodes
                        .iter()
                        .find(|n| n.address == address)
                        .unwrap()
                        .member_owner
                        .is_none()
                );
            }
        }
    }
}

#[test]
fn rest_parent_chain_rejects_indirect_cycles_and_cross_state_resolution() {
    let mut value: Value =
        serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json")).unwrap();
    let mut parent = value["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["address"] == "aws_api_gateway_resource.path")
        .unwrap()
        .clone();
    parent["address"] = json!("aws_api_gateway_resource.parent");
    parent["change"]["after"]["id"] = json!("parent-id");
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(parent);
    let mut config = value["configuration"]["root_module"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["address"] == "aws_api_gateway_resource.path")
        .unwrap()
        .clone();
    config["address"] = json!("aws_api_gateway_resource.parent");
    config["name"] = json!("parent");
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == "aws_api_gateway_resource.path")
        .unwrap()["change"]["after"]["parent_id"] = json!("parent-id");
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == "aws_api_gateway_resource.path")
        .unwrap()["expressions"]["parent_id"] =
        json!({"references":["aws_api_gateway_resource.parent.id"]});
    for cycle in [false, true] {
        if cycle {
            value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_api_gateway_resource.parent")
                .unwrap()["change"]["after"]["parent_id"] = json!("resource-synthetic");
            value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == "aws_api_gateway_resource.parent")
                .unwrap()["expressions"]["parent_id"] =
                json!({"references":["aws_api_gateway_resource.path.id"]});
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        assert_eq!(
            raw.attributes
                .iter()
                .filter(|r| r.attribute == "parent_id" && r.service_reference_verified)
                .count(),
            if cycle { 0 } else { 2 }
        );
    }
    let output = crate::generate_named(
        &[
            crate::NamedInput {
                state: "one".into(),
                json: value.to_string(),
            },
            crate::NamedInput {
                state: "two".into(),
                json: value.to_string(),
            },
        ],
        &crate::Options::default(),
    )
    .unwrap()
    .content;
    assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
}

#[test]
fn rest_authorization_models_and_pools_require_qualified_complete_evidence() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json")).unwrap();
    let authorizer = original["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["type"] == "aws_api_gateway_authorizer" && r["change"]["after"]["type"] == "REQUEST"
        })
        .unwrap()["address"]
        .as_str()
        .unwrap();
    for (address, field) in [
        (authorizer, "authorizer_credentials"),
        ("aws_api_gateway_authorizer.cognito", "provider_arns"),
        ("aws_api_gateway_method.get", "authorizer_id"),
        ("aws_api_gateway_method.get", "request_validator_id"),
        ("aws_api_gateway_method.get", "request_models"),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "wrong_api",
            "wrong_kind",
            "partial",
            "built_in",
            "deleted",
            "wrong_provider",
            "unknown_region",
        ] {
            let mut value = original.clone();
            let source = value["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            match case {
                "unknown" => source["change"]["after_unknown"][field] = json!(true),
                "wrong_api" => source["change"]["after"]["rest_api_id"] = json!("other"),
                "wrong_kind" => {
                    if field == "authorizer_id" {
                        source["change"]["after"]["authorization"] = json!("NONE");
                    } else if field == "provider_arns" || field == "authorizer_credentials" {
                        source["change"]["after"]["type"] = json!("UNKNOWN");
                    }
                }
                "partial" if field == "provider_arns" => source["change"]["after"][field]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unreferenced")),
                "partial" if field == "request_models" => {
                    source["change"]["after"][field]["text/plain"] = json!("Other")
                }
                "deleted" => source["change"]["actions"] = json!(["delete"]),
                "wrong_provider" => source["provider_name"] = json!("example.invalid/custom/aws"),
                _ => {}
            }
            if case == "literal" {
                value["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == address)
                    .unwrap()["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            if case == "unknown_region" {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]});
            }
            if case == "built_in" && field == "request_models" {
                for r in value["resource_changes"].as_array_mut().unwrap() {
                    if r["address"] == address {
                        r["change"]["after"][field] = json!({"application/json":"Empty"});
                    }
                    if r["address"] == "aws_api_gateway_model.response" {
                        r["change"]["after"]["name"] = json!("Empty");
                    }
                }
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let expected = case == "valid"
                || (case == "wrong_kind"
                    && matches!(field, "request_validator_id" | "request_models"))
                || (case == "partial" && !matches!(field, "provider_arns" | "request_models"))
                || (case == "built_in" && field != "request_models");
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field} {case}"
            );
            assert_eq!(
                semantic::transform(&raw).coverage.selected.resources,
                value["resource_changes"].as_array().unwrap().len()
            );
        }
    }
}

#[test]
fn rest_domains_reject_private_id_endpoint_type_and_certificate_mismatches() {
    for (address, field, endpoint) in [
        (
            "aws_api_gateway_domain_name.regional",
            "regional_certificate_arn",
            "aws_acm_certificate.domain",
        ),
        (
            "aws_api_gateway_domain_name.edge",
            "certificate_arn",
            "aws_acm_certificate.domain",
        ),
        (
            "aws_api_gateway_base_path_mapping.private",
            "domain_name",
            "aws_api_gateway_domain_name.private",
        ),
        (
            "aws_api_gateway_base_path_mapping.private",
            "stage_name",
            "aws_api_gateway_stage.live",
        ),
        (
            "aws_api_gateway_domain_name_access_association.private",
            "access_association_source",
            "aws_vpc_endpoint.domain",
        ),
        (
            "aws_api_gateway_domain_name_access_association.private",
            "domain_name_arn",
            "aws_api_gateway_domain_name.private",
        ),
    ] {
        for case in [
            "valid",
            "unknown",
            "literal",
            "deleted",
            "data",
            "wrong_type",
            "private_id",
            "other_api",
        ] {
            let mut v: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let target = v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            if case == "unknown" {
                target["change"]["after_unknown"][field] = json!(true);
            }
            if case == "private_id" && address.contains("base_path_mapping") {
                target["change"]["after"]["domain_name_id"] = json!("other");
            }
            if case == "wrong_type" {
                if address.starts_with("aws_api_gateway_domain_name.") {
                    target["change"]["after"]["endpoint_configuration"] =
                        json!([{"types":["UNKNOWN"]}]);
                } else if field == "access_association_source" {
                    target["change"]["after"]["access_association_source_type"] = json!("UNKNOWN");
                }
            }
            let source = v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == endpoint)
                .unwrap();
            match case {
                "deleted" => source["change"]["actions"] = json!(["delete"]),
                "data" => source["mode"] = json!("data"),
                "other_api" if field == "stage_name" => {
                    source["change"]["after"]["rest_api_id"] = json!("other")
                }
                "wrong_type" if field == "domain_name_arn" || field == "domain_name" => {
                    source["change"]["after"]["endpoint_configuration"] =
                        json!([{"types":["UNKNOWN"]}])
                }
                _ => {}
            }
            if case == "literal" {
                v["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == address)
                    .unwrap()["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            let raw = plan::parse(&v.to_string()).unwrap();
            let expected = case == "valid"
                || (case == "private_id" && !address.contains("base_path_mapping"))
                || (case == "other_api" && field != "stage_name")
                || (case == "wrong_type" && field == "stage_name");
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field} {case}"
            );
            assert_eq!(
                semantic::transform(&raw).coverage.selected.resources,
                v["resource_changes"].as_array().unwrap().len()
            );
        }
    }
}

#[test]
fn v2_domains_and_rest_routing_require_single_qualified_actions() {
    for (address, field) in [
        (
            "aws_apigatewayv2_domain_name.mapping",
            "domain_name_configuration.certificate_arn",
        ),
        ("aws_apigatewayv2_api_mapping.http", "stage"),
        (
            "aws_apigatewayv2_routing_rule.rest",
            "action.invoke_api.api_id",
        ),
        (
            "aws_apigatewayv2_routing_rule.rest",
            "action.invoke_api.stage",
        ),
    ] {
        for case in [
            "valid",
            "unknown",
            "literal",
            "wrong_api",
            "extra_block",
            "wrong_domain",
            "deleted",
        ] {
            let mut v: Value =
                serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                    .unwrap();
            let target = v["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            let root = field.split('.').next().unwrap();
            if case == "unknown" {
                target["change"]["after_unknown"][root] = json!(true);
            }
            if case == "extra_block" && field.contains('.') {
                let array = target["change"]["after"][root].as_array_mut().unwrap();
                array.push(array[0].clone());
            }
            if case == "deleted" {
                target["change"]["actions"] = json!(["delete"]);
            }
            if case == "literal" {
                v["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == address)
                    .unwrap()["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove(root);
            }
            if case == "wrong_domain" {
                v["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["address"] == "aws_apigatewayv2_domain_name.mapping")
                    .unwrap()["change"]["after"]["domain_name_configuration"][0]["endpoint_type"] =
                    json!("EDGE");
            }
            if case == "wrong_api" {
                if address.contains("api_mapping") {
                    v["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["address"] == "aws_apigatewayv2_stage.mapping")
                        .unwrap()["change"]["after"]["api_id"] = json!("other");
                } else if field.ends_with("stage") {
                    v["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["address"] == "aws_api_gateway_stage.live")
                        .unwrap()["change"]["after"]["rest_api_id"] = json!("other");
                }
            }
            let raw = plan::parse(&v.to_string()).unwrap();
            let expected = case == "valid"
                || (case == "extra_block" && !field.contains('.'))
                || (case == "wrong_api" && !field.ends_with("stage"));
            assert_eq!(
                raw.attributes
                    .iter()
                    .any(|r| raw.nodes[r.target].address == address
                        && r.attribute == field
                        && r.service_reference_verified),
                expected,
                "{address} {field} {case}"
            );
            assert_eq!(
                semantic::transform(&raw).coverage.selected.resources,
                v["resource_changes"].as_array().unwrap().len()
            );
        }
    }
}

#[test]
fn public_routing_rejects_private_unknown_and_missing_rest_endpoint_types() {
    for case in [
        "REGIONAL", "EDGE", "PRIVATE", "unknown", "missing", "multiple",
    ] {
        let mut v: Value =
            serde_json::from_str(include_str!("../../fixtures/api-gateway-rest-plan.json"))
                .unwrap();
        let api = v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == "aws_api_gateway_rest_api.app")
            .unwrap();
        api["change"]["after"]["endpoint_configuration"] = json!([{"types":[case]}]);
        if case == "unknown" {
            api["change"]["after_unknown"]["endpoint_configuration"] = json!(true);
        }
        if case == "missing" {
            api["change"]["after"]
                .as_object_mut()
                .unwrap()
                .remove("endpoint_configuration");
        }
        if case == "multiple" {
            api["change"]["after"]["endpoint_configuration"] =
                json!([{"types":["REGIONAL","PRIVATE"]}]);
        }
        let raw = plan::parse(&v.to_string()).unwrap();
        for field in ["action.invoke_api.api_id", "action.invoke_api.stage"] {
            assert_eq!(
                raw.attributes.iter().any(|r| raw.nodes[r.target].address
                    == "aws_apigatewayv2_routing_rule.rest"
                    && r.attribute == field
                    && r.service_reference_verified),
                matches!(case, "REGIONAL" | "EDGE"),
                "{field}: {case}"
            );
        }
    }
}
