use crate::{plan, semantic};
use serde_json::{Value, json};

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
