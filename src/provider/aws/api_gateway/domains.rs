use super::{Context, Rule};
use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets): (_, &[_]) = match (resource, attribute) {
        ("aws_apigatewayv2_api_mapping", "api_id") => (
            "Mapped API",
            &[
                ("aws_api_gateway_rest_api", "id"),
                ("aws_apigatewayv2_api", "id"),
            ],
        ),
        ("aws_apigatewayv2_api_mapping" | "aws_apigatewayv2_routing_rule", "domain_name") => (
            "Routing domain",
            &[
                ("aws_api_gateway_domain_name", "domain_name"),
                ("aws_apigatewayv2_domain_name", "domain_name"),
                ("aws_apigatewayv2_domain_name", "id"),
            ],
        ),
        ("aws_apigatewayv2_api_mapping", "stage") => (
            "Mapped stage",
            &[
                ("aws_api_gateway_stage", "stage_name"),
                ("aws_apigatewayv2_stage", "name"),
                ("aws_apigatewayv2_stage", "id"),
            ],
        ),
        (
            "aws_apigatewayv2_domain_name",
            "domain_name_configuration.certificate_arn"
            | "domain_name_configuration.ownership_verification_certificate_arn",
        ) => ("V2 domain certificate", &[("aws_acm_certificate", "arn")]),
        ("aws_apigatewayv2_routing_rule", "action.invoke_api.api_id") => {
            ("Routing REST API", &[("aws_api_gateway_rest_api", "id")])
        }
        ("aws_apigatewayv2_routing_rule", "action.invoke_api.stage") => (
            "Routing REST stage",
            &[("aws_api_gateway_stage", "stage_name")],
        ),
        ("aws_api_gateway_base_path_mapping", "api_id") => {
            ("REST mapped API", &[("aws_api_gateway_rest_api", "id")])
        }
        ("aws_api_gateway_base_path_mapping", "domain_name") => (
            "REST mapped domain",
            &[("aws_api_gateway_domain_name", "domain_name")],
        ),
        ("aws_api_gateway_base_path_mapping", "domain_name_id") => (
            "REST private domain ID",
            &[("aws_api_gateway_domain_name", "domain_name_id")],
        ),
        ("aws_api_gateway_base_path_mapping", "stage_name") => (
            "REST mapped stage",
            &[("aws_api_gateway_stage", "stage_name")],
        ),
        (
            "aws_api_gateway_domain_name",
            "certificate_arn"
            | "regional_certificate_arn"
            | "ownership_verification_certificate_arn",
        ) => ("REST domain certificate", &[("aws_acm_certificate", "arn")]),
        ("aws_api_gateway_domain_name_access_association", "domain_name_arn") => (
            "REST private domain",
            &[("aws_api_gateway_domain_name", "arn")],
        ),
        ("aws_api_gateway_domain_name_access_association", "access_association_source") => {
            ("REST domain VPC endpoint", &[("aws_vpc_endpoint", "id")])
        }
        _ => return None,
    };
    Some(Rule {
        label,
        targets,
        list: false,
    })
}

fn endpoint_type<'a>(context: &'a Context<'_>, index: usize) -> Option<&'a str> {
    let (value, unknown) = context.evidence[index];
    if value["endpoint_configuration"].as_array()?.len() != 1 {
        return None;
    }
    let types =
        super::super::streaming::values(value, unknown, "endpoint_configuration.types", true)?;
    if types.len() != 1 {
        return None;
    }
    types.into_iter().next()
}

fn mapping_domain(context: &Context<'_>, index: usize) -> bool {
    context.source(index, "domain_name").is_some_and(|domain| {
        match endpoint_type(context, domain) {
            Some("PRIVATE") => context.source(index, "domain_name_id") == Some(domain),
            Some("EDGE" | "REGIONAL") => {
                let (value, unknown) = context.evidence[index];
                matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                    && matches!(&unknown["domain_name_id"], Value::Null | Value::Bool(false))
                    && (value["domain_name_id"].is_null() || value["domain_name_id"] == "")
            }
            _ => false,
        }
    })
}

fn regional_domain(context: &Context<'_>, index: usize) -> bool {
    match context.nodes[index].resource_type.as_str() {
        "aws_api_gateway_domain_name" => endpoint_type(context, index) == Some("REGIONAL"),
        "aws_apigatewayv2_domain_name" => {
            let (value, unknown) = context.evidence[index];
            value["domain_name_configuration"]
                .as_array()
                .is_some_and(|v| v.len() == 1)
                && super::super::streaming::values(
                    value,
                    unknown,
                    "domain_name_configuration.endpoint_type",
                    false,
                )
                .is_some_and(|v| v.len() == 1 && v.contains("REGIONAL"))
        }
        _ => false,
    }
}

fn single_action(context: &Context<'_>, index: usize) -> bool {
    context.evidence[index].0["action"]
        .as_array()
        .is_some_and(|a| {
            a.len() == 1 && a[0]["invoke_api"].as_array().is_some_and(|i| i.len() == 1)
        })
}

fn public_rest_api(context: &Context<'_>, index: usize) -> bool {
    matches!(endpoint_type(context, index), Some("EDGE" | "REGIONAL"))
}

pub(super) fn edge_eligible(nodes: &[TerraformEntity], r: &AttributeReference) -> bool {
    if nodes[r.target].resource_type != "aws_api_gateway_domain_name"
        || r.attribute != "certificate_arn"
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.len() != 1
    {
        return false;
    }
    let source = &nodes[r.sources[0]];
    let target = &nodes[r.target];
    [source, target].iter().all(|n| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    }) && source.resource_type == "aws_acm_certificate"
        && super::super::scope::scope(source)
            .and_then(|s| s.region)
            .is_some_and(|r| r.0 == "us-east-1")
        && super::super::eventbridge::bindings(
            target.provider_configuration.as_ref(),
            source.provider_configuration.as_ref(),
            super::super::scope::scope(target)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            super::super::scope::scope(source)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            true,
        )
}

pub(super) fn valid(context: &Context<'_>, r: &AttributeReference) -> bool {
    let source = r.sources[0];
    match (
        context.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (
            "aws_apigatewayv2_domain_name",
            "domain_name_configuration.certificate_arn"
            | "domain_name_configuration.ownership_verification_certificate_arn",
        ) => regional_domain(context, r.target),
        ("aws_apigatewayv2_api_mapping" | "aws_apigatewayv2_routing_rule", "domain_name") => {
            regional_domain(context, source)
        }
        ("aws_apigatewayv2_api_mapping", "api_id") => {
            (context.nodes[source].resource_type == "aws_api_gateway_rest_api"
                && public_rest_api(context, source))
                || matches!(
                    context.value(source, "protocol_type"),
                    Some("HTTP" | "WEBSOCKET")
                )
        }
        ("aws_apigatewayv2_api_mapping", "stage") => {
            context
                .source(r.target, "domain_name")
                .is_some_and(|d| regional_domain(context, d))
                && context.source(r.target, "api_id").is_some_and(|api| {
                    match context.nodes[api].resource_type.as_str() {
                        "aws_api_gateway_rest_api" => {
                            public_rest_api(context, api)
                                && context.nodes[source].resource_type == "aws_api_gateway_stage"
                                && context.source(source, "rest_api_id") == Some(api)
                        }
                        "aws_apigatewayv2_api" => {
                            matches!(
                                context.value(api, "protocol_type"),
                                Some("HTTP" | "WEBSOCKET")
                            ) && context.nodes[source].resource_type == "aws_apigatewayv2_stage"
                                && context.source(source, "api_id") == Some(api)
                        }
                        _ => false,
                    }
                })
        }
        ("aws_apigatewayv2_routing_rule", "action.invoke_api.api_id") => {
            single_action(context, r.target)
                && public_rest_api(context, source)
                && context
                    .source(r.target, "domain_name")
                    .is_some_and(|d| regional_domain(context, d))
        }
        ("aws_apigatewayv2_routing_rule", "action.invoke_api.stage") => {
            single_action(context, r.target)
                && context
                    .source(r.target, "domain_name")
                    .is_some_and(|d| regional_domain(context, d))
                && context
                    .source(r.target, "action.invoke_api.api_id")
                    .is_some_and(|api| {
                        public_rest_api(context, api)
                            && context.source(source, "rest_api_id") == Some(api)
                    })
        }
        ("aws_api_gateway_base_path_mapping", "domain_name" | "domain_name_id") => {
            mapping_domain(context, r.target)
        }
        ("aws_api_gateway_base_path_mapping", "stage_name") => {
            mapping_domain(context, r.target)
                && context
                    .source(r.target, "api_id")
                    .is_some_and(|api| context.source(source, "rest_api_id") == Some(api))
        }
        ("aws_api_gateway_domain_name", "certificate_arn") => {
            (endpoint_type(context, r.target) == Some("PRIVATE")
                && super::super::service_evidence::eligible(context.nodes, r))
                || (endpoint_type(context, r.target) == Some("EDGE")
                    && super::super::scope::scope(&context.nodes[source])
                        .and_then(|s| s.region)
                        .is_some_and(|r| r.0 == "us-east-1"))
        }
        ("aws_api_gateway_domain_name", "regional_certificate_arn") => {
            matches!(endpoint_type(context, r.target), Some("REGIONAL"))
        }
        ("aws_api_gateway_domain_name", "ownership_verification_certificate_arn") => {
            endpoint_type(context, r.target) == Some("REGIONAL")
        }
        ("aws_api_gateway_domain_name_access_association", "domain_name_arn") => {
            endpoint_type(context, source) == Some("PRIVATE")
        }
        ("aws_api_gateway_domain_name_access_association", "access_association_source") => {
            context.value(r.target, "access_association_source_type") == Some("VPCE")
                && context.value(source, "vpc_endpoint_type") == Some("Interface")
                && context
                    .source(r.target, "domain_name_arn")
                    .is_some_and(|domain| endpoint_type(context, domain) == Some("PRIVATE"))
        }
        _ => true,
    }
}
