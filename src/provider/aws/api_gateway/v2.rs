use super::{Context, Rule};
use crate::model::AttributeReference;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets, list): (_, &[_], _) = match (resource, attribute) {
        (
            "aws_apigatewayv2_integration"
            | "aws_apigatewayv2_stage"
            | "aws_apigatewayv2_authorizer"
            | "aws_apigatewayv2_deployment",
            "api_id",
        ) => ("V2 API context", &[("aws_apigatewayv2_api", "id")], false),
        ("aws_apigatewayv2_stage", "deployment_id") => (
            "V2 explicit deployment",
            &[("aws_apigatewayv2_deployment", "id")],
            false,
        ),
        ("aws_apigatewayv2_integration", "connection_id") => {
            ("V2 VPC link", &[("aws_apigatewayv2_vpc_link", "id")], false)
        }
        ("aws_apigatewayv2_integration", "integration_uri") => (
            "V2 integration backend",
            &[
                ("aws_lambda_function", "invoke_arn"),
                ("aws_lambda_function", "arn"),
                ("aws_lambda_alias", "invoke_arn"),
                ("aws_lambda_alias", "arn"),
                ("aws_lb_listener", "arn"),
            ],
            false,
        ),
        ("aws_apigatewayv2_authorizer", "authorizer_uri") => (
            "V2 Lambda authorizer",
            &[
                ("aws_lambda_function", "invoke_arn"),
                ("aws_lambda_alias", "invoke_arn"),
            ],
            false,
        ),
        ("aws_apigatewayv2_vpc_link", "subnet_ids") => {
            ("V2 link subnets", &[("aws_subnet", "id")], true)
        }
        ("aws_apigatewayv2_vpc_link", "security_group_ids") => (
            "V2 link security groups",
            &[("aws_security_group", "id")],
            true,
        ),
        _ => return None,
    };
    Some(Rule {
        label,
        targets,
        list,
    })
}

pub(super) fn valid(context: &Context<'_>, r: &AttributeReference) -> bool {
    let source = r.sources[0];
    let api = context.source(r.target, "api_id");
    let protocol = api.and_then(|api| context.value(api, "protocol_type"));
    match (
        context.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_apigatewayv2_stage", "deployment_id") => {
            context.known_false(r.target, "auto_deploy")
                && api.is_some_and(|api| context.source(source, "api_id") == Some(api))
        }
        ("aws_apigatewayv2_integration", "connection_id") => {
            protocol == Some("HTTP")
                && context.value(r.target, "connection_type") == Some("VPC_LINK")
                && context.value(r.target, "integration_type") == Some("HTTP_PROXY")
        }
        ("aws_apigatewayv2_integration", "integration_uri") => {
            if !matches!(protocol, Some("HTTP" | "WEBSOCKET")) {
                return false;
            }
            if context.lambda(source) {
                context.value(r.target, "integration_type") == Some("AWS_PROXY")
                    && (r.source_attributes.contains("invoke_arn") || protocol == Some("HTTP"))
                    && context.value(r.target, "connection_type") == Some("INTERNET")
            } else {
                protocol == Some("HTTP")
                    && context.value(r.target, "integration_type") == Some("HTTP_PROXY")
                    && context.value(r.target, "connection_type") == Some("VPC_LINK")
                    && context.source(r.target, "connection_id").is_some()
                    && context.listener(source)
            }
        }
        ("aws_apigatewayv2_authorizer", "authorizer_uri") => {
            matches!(protocol, Some("HTTP" | "WEBSOCKET"))
                && context.value(r.target, "authorizer_type") == Some("REQUEST")
                && context.lambda(source)
        }
        _ => true,
    }
}
