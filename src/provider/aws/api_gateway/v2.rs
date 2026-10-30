use super::{Context, Rule};
use crate::model::AttributeReference;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets, list): (_, &[_], _) = match (resource, attribute) {
        ("aws_apigatewayv2_api", "target") => (
            "V2 quick-create Lambda",
            &[("aws_lambda_function", "arn"), ("aws_lambda_alias", "arn")],
            false,
        ),
        ("aws_apigatewayv2_api" | "aws_apigatewayv2_integration", "credentials_arn") => {
            ("V2 integration role", &[("aws_iam_role", "arn")], false)
        }
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
                ("aws_service_discovery_service", "arn"),
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
        ("aws_apigatewayv2_api", "target") => {
            context.value(r.target, "protocol_type") == Some("HTTP") && context.lambda(source)
        }
        ("aws_apigatewayv2_api", "credentials_arn") => {
            context.value(r.target, "protocol_type") == Some("HTTP")
                && context
                    .source(r.target, "target")
                    .is_some_and(|target| context.lambda(target))
        }
        ("aws_apigatewayv2_integration", "credentials_arn") => {
            matches!(
                (protocol, context.value(r.target, "integration_type")),
                (Some("HTTP" | "WEBSOCKET"), Some("AWS_PROXY")) | (Some("WEBSOCKET"), Some("AWS"))
            )
        }
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
                let (value, unknown) = context.evidence[r.target];
                let no_subtype = matches!(
                    unknown,
                    serde_json::Value::Null
                        | serde_json::Value::Bool(false)
                        | serde_json::Value::Object(_)
                ) && matches!(
                    &unknown["integration_subtype"],
                    serde_json::Value::Null | serde_json::Value::Bool(false)
                ) && (value["integration_subtype"].is_null()
                    || value["integration_subtype"] == "");
                context.value(r.target, "integration_type") == Some("AWS_PROXY")
                    && no_subtype
                    && (r.source_attributes.contains("invoke_arn") || protocol == Some("HTTP"))
                    && context.value(r.target, "connection_type") == Some("INTERNET")
            } else {
                protocol == Some("HTTP")
                    && context.value(r.target, "integration_type") == Some("HTTP_PROXY")
                    && context.value(r.target, "connection_type") == Some("VPC_LINK")
                    && context.source(r.target, "connection_id").is_some()
                    && (context.listener(source)
                        || context.nodes[source].resource_type == "aws_service_discovery_service")
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
