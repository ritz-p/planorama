use super::{Context, Rule};
use crate::model::AttributeReference;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets, list): (_, &[_], _) = match (resource, attribute) {
        (
            "aws_api_gateway_integration"
            | "aws_api_gateway_stage"
            | "aws_api_gateway_authorizer"
            | "aws_api_gateway_resource"
            | "aws_api_gateway_method"
            | "aws_api_gateway_deployment",
            "rest_api_id",
        ) => (
            "REST API context",
            &[("aws_api_gateway_rest_api", "id")],
            false,
        ),
        ("aws_api_gateway_integration" | "aws_api_gateway_method", "resource_id") => (
            "REST resource context",
            &[
                ("aws_api_gateway_resource", "id"),
                ("aws_api_gateway_rest_api", "root_resource_id"),
            ],
            false,
        ),
        ("aws_api_gateway_integration", "http_method") => (
            "REST method context",
            &[("aws_api_gateway_method", "http_method")],
            false,
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
        ("aws_api_gateway_integration" | "aws_api_gateway_method", "resource_id") => {
            resource_matches(context, r.target, source)
        }
        ("aws_api_gateway_integration", "http_method") => method_matches(context, r.target, source),
        ("aws_api_gateway_stage", "deployment_id") => context
            .source(r.target, "rest_api_id")
            .is_some_and(|api| context.source(source, "rest_api_id") == Some(api)),
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
