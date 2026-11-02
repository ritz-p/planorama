use super::{Context, Rule};
use crate::model::AttributeReference;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets, list): (_, &[_], _) = match (resource, attribute) {
        ("aws_api_gateway_authorizer", "authorizer_credentials") => {
            ("REST authorizer role", &[("aws_iam_role", "arn")], false)
        }
        ("aws_api_gateway_authorizer", "provider_arns") => (
            "REST Cognito pools",
            &[("aws_cognito_user_pool", "arn")],
            true,
        ),
        ("aws_api_gateway_method", "authorizer_id") => (
            "REST method authorizer",
            &[("aws_api_gateway_authorizer", "id")],
            false,
        ),
        ("aws_api_gateway_method", "request_validator_id") => (
            "REST request validator",
            &[("aws_api_gateway_request_validator", "id")],
            false,
        ),
        ("aws_api_gateway_method", "request_models") => (
            "REST request models",
            &[("aws_api_gateway_model", "name")],
            true,
        ),
        ("aws_api_gateway_account", "cloudwatch_role_arn") => (
            "API Gateway logging role",
            &[("aws_iam_role", "arn")],
            false,
        ),
        ("aws_api_gateway_stage", "client_certificate_id") => (
            "REST client certificate",
            &[("aws_api_gateway_client_certificate", "id")],
            false,
        ),
        ("aws_api_gateway_stage", "documentation_version") => (
            "REST documentation version",
            &[("aws_api_gateway_documentation_version", "version")],
            false,
        ),
        ("aws_api_gateway_stage", "canary_settings.deployment_id") => (
            "REST canary deployment",
            &[("aws_api_gateway_deployment", "id")],
            false,
        ),
        ("aws_api_gateway_stage", "access_log_settings.destination_arn") => (
            "REST access logs",
            &[
                ("aws_cloudwatch_log_group", "arn"),
                ("aws_kinesis_firehose_delivery_stream", "arn"),
            ],
            false,
        ),
        ("aws_api_gateway_rest_api", "endpoint_configuration.vpc_endpoint_ids") => (
            "Private REST endpoints",
            &[("aws_vpc_endpoint", "id")],
            true,
        ),
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_stage"
            | "aws_api_gateway_authorizer"
            | "aws_api_gateway_resource"
            | "aws_api_gateway_method"
            | "aws_api_gateway_deployment"
            | "aws_api_gateway_documentation_part"
            | "aws_api_gateway_gateway_response"
            | "aws_api_gateway_integration_response"
            | "aws_api_gateway_method_response"
            | "aws_api_gateway_method_settings"
            | "aws_api_gateway_model"
            | "aws_api_gateway_request_validator"
            | "aws_api_gateway_rest_api_policy"
            | "aws_api_gateway_rest_api_put"
            | "aws_api_gateway_documentation_version",
            "rest_api_id",
        ) => (
            "REST API context",
            &[("aws_api_gateway_rest_api", "id")],
            false,
        ),
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_method"
            | "aws_api_gateway_method_response"
            | "aws_api_gateway_integration_response",
            "resource_id",
        ) => (
            "REST resource context",
            &[
                ("aws_api_gateway_resource", "id"),
                ("aws_api_gateway_rest_api", "root_resource_id"),
            ],
            false,
        ),
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_method_response"
            | "aws_api_gateway_integration_response",
            "http_method",
        ) => (
            "REST method context",
            &[("aws_api_gateway_method", "http_method")],
            false,
        ),
        ("aws_api_gateway_resource", "parent_id") => (
            "REST parent resource",
            &[
                ("aws_api_gateway_resource", "id"),
                ("aws_api_gateway_rest_api", "root_resource_id"),
            ],
            false,
        ),
        ("aws_api_gateway_method_settings", "stage_name") => (
            "REST settings stage",
            &[("aws_api_gateway_stage", "stage_name")],
            false,
        ),
        ("aws_api_gateway_integration_response", "status_code") => (
            "REST method response",
            &[("aws_api_gateway_method_response", "status_code")],
            false,
        ),
        ("aws_api_gateway_method_response", "response_models") => (
            "REST response models",
            &[("aws_api_gateway_model", "name")],
            true,
        ),
        ("aws_api_gateway_stage", "deployment_id") => (
            "REST deployment",
            &[("aws_api_gateway_deployment", "id")],
            false,
        ),
        ("aws_api_gateway_integration", "connection_id") => (
            "REST VPC link",
            &[
                ("aws_api_gateway_vpc_link", "id"),
                ("aws_apigatewayv2_vpc_link", "id"),
            ],
            false,
        ),
        ("aws_api_gateway_integration", "integration_target") => {
            ("REST private backend", &[("aws_lb", "arn")], false)
        }
        ("aws_api_gateway_integration", "credentials") => {
            ("REST integration role", &[("aws_iam_role", "arn")], false)
        }
        ("aws_api_gateway_integration", "uri") => (
            "REST Lambda backend",
            &[
                ("aws_lambda_function", "invoke_arn"),
                ("aws_lambda_alias", "invoke_arn"),
            ],
            false,
        ),
        ("aws_api_gateway_authorizer", "authorizer_uri") => (
            "REST Lambda authorizer",
            &[
                ("aws_lambda_function", "invoke_arn"),
                ("aws_lambda_alias", "invoke_arn"),
            ],
            false,
        ),
        ("aws_api_gateway_vpc_link", "target_arns") => {
            ("REST NLB targets", &[("aws_lb", "arn")], true)
        }
        _ => return None,
    };
    Some(Rule {
        label,
        targets,
        list,
    })
}

fn resource_matches(context: &Context<'_>, owner: usize, resource: usize) -> bool {
    context.source(owner, "rest_api_id").is_some_and(|api| {
        match context.nodes[resource].resource_type.as_str() {
            "aws_api_gateway_rest_api" => resource == api,
            "aws_api_gateway_resource" => context.source(resource, "rest_api_id") == Some(api),
            _ => false,
        }
    })
}

fn method_matches(context: &Context<'_>, owner: usize, method: usize) -> bool {
    context.source(owner, "rest_api_id").is_some_and(|api| {
        context.source(method, "rest_api_id") == Some(api)
            && context
                .source(owner, "resource_id")
                .is_some_and(|resource| {
                    resource_matches(context, owner, resource)
                        && context.source(method, "resource_id") == Some(resource)
                })
    })
}

pub(super) fn valid(context: &Context<'_>, r: &AttributeReference) -> bool {
    let source = r.sources[0];
    match (
        context.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_api_gateway_authorizer", "authorizer_credentials") => {
            context.source(r.target, "rest_api_id").is_some()
                && matches!(context.value(r.target, "type"), Some("TOKEN" | "REQUEST"))
        }
        ("aws_api_gateway_authorizer", "provider_arns") => {
            context.source(r.target, "rest_api_id").is_some()
                && context.value(r.target, "type") == Some("COGNITO_USER_POOLS")
        }
        ("aws_api_gateway_method", "authorizer_id" | "request_validator_id" | "request_models") => {
            context
                .source(r.target, "resource_id")
                .is_some_and(|resource| resource_matches(context, r.target, resource))
                && context.source(r.target, "rest_api_id").is_some_and(|api| {
                    r.sources
                        .iter()
                        .all(|&endpoint| context.source(endpoint, "rest_api_id") == Some(api))
                })
                && match r.attribute.as_str() {
                    "authorizer_id" => matches!(
                        (
                            context.value(r.target, "authorization"),
                            context.value(source, "type")
                        ),
                        (Some("CUSTOM"), Some("TOKEN" | "REQUEST"))
                            | (Some("COGNITO_USER_POOLS"), Some("COGNITO_USER_POOLS"))
                    ),
                    "request_models" => r.sources.iter().all(|&model| {
                        !matches!(context.value(model, "name"), Some("Empty" | "Error"))
                    }),
                    _ => true,
                }
        }
        ("aws_api_gateway_resource", "parent_id") => {
            let mut seen = std::collections::BTreeSet::from([r.target]);
            let mut parent = source;
            loop {
                if !seen.insert(parent) || !resource_matches(context, r.target, parent) {
                    return false;
                }
                if context.nodes[parent].resource_type == "aws_api_gateway_rest_api" {
                    return true;
                }
                let Some(next) = context.source(parent, "parent_id") else {
                    return false;
                };
                parent = next;
            }
        }
        ("aws_api_gateway_method_settings", "stage_name") => context
            .source(r.target, "rest_api_id")
            .is_some_and(|api| context.source(source, "rest_api_id") == Some(api)),
        ("aws_api_gateway_method_response", "response_models") => {
            context
                .source(r.target, "http_method")
                .is_some_and(|method| method_matches(context, r.target, method))
                && context.source(r.target, "rest_api_id").is_some_and(|api| {
                    r.sources.iter().all(|&model| {
                        context.source(model, "rest_api_id") == Some(api)
                            && !matches!(context.value(model, "name"), Some("Empty" | "Error"))
                    })
                })
        }
        ("aws_api_gateway_integration_response", "status_code") => context
            .source(r.target, "http_method")
            .is_some_and(|method| {
                method_matches(context, r.target, method)
                    && context.source(source, "http_method") == Some(method)
                    && method_matches(context, source, method)
            }),
        ("aws_api_gateway_rest_api", "endpoint_configuration.vpc_endpoint_ids") => {
            let (value, unknown) = context.evidence[r.target];
            value["endpoint_configuration"]
                .as_array()
                .is_some_and(|blocks| blocks.len() == 1)
                && super::super::streaming::values(
                    value,
                    unknown,
                    "endpoint_configuration.types",
                    true,
                )
                .is_some_and(|types| types.len() == 1 && types.contains("PRIVATE"))
                && r.sources.iter().all(|&endpoint| {
                    context.value(endpoint, "vpc_endpoint_type") == Some("Interface")
                })
        }
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_method"
            | "aws_api_gateway_method_response"
            | "aws_api_gateway_integration_response",
            "resource_id",
        ) => resource_matches(context, r.target, source),
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_method_response"
            | "aws_api_gateway_integration_response",
            "http_method",
        ) => method_matches(context, r.target, source),
        (
            "aws_api_gateway_stage",
            "deployment_id" | "canary_settings.deployment_id" | "documentation_version",
        ) => context
            .source(r.target, "rest_api_id")
            .is_some_and(|api| context.source(source, "rest_api_id") == Some(api)),
        ("aws_api_gateway_stage", "client_certificate_id") => {
            context.source(r.target, "rest_api_id").is_some()
        }
        ("aws_api_gateway_stage", "access_log_settings.destination_arn") => {
            context.source(r.target, "rest_api_id").is_some()
                && (context.nodes[source].resource_type != "aws_kinesis_firehose_delivery_stream"
                    || context
                        .value(source, "name")
                        .is_some_and(|name| name.starts_with("amazon-apigateway-")))
        }
        ("aws_api_gateway_integration", "uri") => {
            matches!(context.value(r.target, "type"), Some("AWS" | "AWS_PROXY"))
                && context.value(r.target, "integration_http_method") == Some("POST")
                && context.lambda(source)
                && context
                    .source(r.target, "http_method")
                    .is_some_and(|method| method_matches(context, r.target, method))
        }
        ("aws_api_gateway_integration", "credentials") => {
            matches!(context.value(r.target, "type"), Some("AWS" | "AWS_PROXY"))
                && context
                    .source(r.target, "http_method")
                    .is_some_and(|method| method_matches(context, r.target, method))
        }
        ("aws_api_gateway_integration", "integration_target") => {
            context.value(r.target, "connection_type") == Some("VPC_LINK")
                && matches!(context.value(r.target, "type"), Some("HTTP" | "HTTP_PROXY"))
                && matches!(
                    context.value(source, "load_balancer_type"),
                    Some("application" | "network")
                )
                && context
                    .source(r.target, "connection_id")
                    .is_some_and(|link| {
                        context.nodes[link].resource_type == "aws_apigatewayv2_vpc_link"
                    })
                && context
                    .source(r.target, "http_method")
                    .is_some_and(|method| method_matches(context, r.target, method))
        }
        ("aws_api_gateway_integration", "connection_id") => {
            context.value(r.target, "connection_type") == Some("VPC_LINK")
                && matches!(context.value(r.target, "type"), Some("HTTP" | "HTTP_PROXY"))
                && context
                    .source(r.target, "http_method")
                    .is_some_and(|method| method_matches(context, r.target, method))
        }
        ("aws_api_gateway_authorizer", "authorizer_uri") => {
            matches!(context.value(r.target, "type"), Some("TOKEN" | "REQUEST"))
                && context.source(r.target, "rest_api_id").is_some()
                && context.lambda(source)
        }
        ("aws_api_gateway_vpc_link", "target_arns") => {
            r.sources.len() == 1 && context.value(source, "load_balancer_type") == Some("network")
        }
        _ => true,
    }
}
