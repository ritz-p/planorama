use super::{Context, Rule};
use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    let (label, targets): (_, &[_]) = match (resource, attribute) {
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
