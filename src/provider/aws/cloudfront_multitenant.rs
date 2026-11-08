use crate::model::{AttributeReference, TerraformEntity, WafScope};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront_multitenant.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudfront_multitenant_distribution"
            | "aws_cloudfront_distribution_tenant"
            | "aws_cloudfront_connection_group"
    )
}
fn consumer_path(path: &str) -> Option<String> {
    if matches!(
        path,
        "origin.domain_name"
            | "origin.origin_access_control_id"
            | "origin.vpc_origin_config.vpc_origin_id"
            | "origin.custom_origin_config.origin_mtls_config.client_certificate_arn"
    ) {
        return Some(path.into());
    }
    let (prefix, field) = path.split_once('.')?;
    if !matches!(prefix, "default_cache_behavior" | "cache_behavior")
        || !matches!(
            field,
            "cache_policy_id"
                | "lambda_function_association.lambda_function_arn"
                | "function_association.function_arn"
                | "trusted_key_groups.items"
                | "field_level_encryption_id"
                | "realtime_log_config_arn"
        )
    {
        return None;
    }
    Some(format!(
        "{}.{}",
        if prefix == "cache_behavior" {
            "ordered_cache_behavior"
        } else {
            prefix
        },
        field
            .replace("lambda_function_arn", "lambda_arn")
            .replace("trusted_key_groups.items", "trusted_key_groups")
    ))
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_cloudfront_distribution_tenant", "distribution_id") => {
            Some("Multitenant distribution")
        }
        ("aws_cloudfront_distribution_tenant", "connection_group_id") => {
            Some("Tenant connection group")
        }
        ("aws_cloudfront_distribution_tenant", "customizations.certificate.arn")
        | ("aws_cloudfront_multitenant_distribution", "viewer_certificate.acm_certificate_arn") => {
            Some("Viewer certificate reference")
        }
        ("aws_cloudfront_distribution_tenant", "customizations.web_acl.arn")
        | ("aws_cloudfront_multitenant_distribution", "web_acl_id") => {
            Some("CloudFront WAF reference")
        }
        ("aws_cloudfront_connection_group", "anycast_ip_list_id") => {
            Some("Anycast IP list reference")
        }
        ("aws_cloudfront_multitenant_distribution", _) => {
            consumer_path(path).and_then(|p| super::cloudfront::label(&p))
        }
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (
            "aws_cloudfront_distribution_tenant",
            "distribution_id",
            "aws_cloudfront_multitenant_distribution",
        )
        | (
            "aws_cloudfront_distribution_tenant",
            "connection_group_id",
            "aws_cloudfront_connection_group",
        )
        | (
            "aws_cloudfront_connection_group",
            "anycast_ip_list_id",
            "aws_cloudfront_anycast_ip_list",
        ) => &["id"],
        (
            "aws_cloudfront_distribution_tenant",
            "customizations.certificate.arn",
            "aws_acm_certificate",
        )
        | (
            "aws_cloudfront_multitenant_distribution",
            "viewer_certificate.acm_certificate_arn",
            "aws_acm_certificate",
        )
        | (
            "aws_cloudfront_distribution_tenant",
            "customizations.web_acl.arn",
            "aws_wafv2_web_acl",
        )
        | ("aws_cloudfront_multitenant_distribution", "web_acl_id", "aws_wafv2_web_acl") => {
            &["arn"]
        }
        ("aws_cloudfront_multitenant_distribution", _, _) => {
            consumer_path(path).map_or(&[], |p| super::cloudfront::source_fields(&p, source))
        }
        _ => &[],
    }
}
pub(super) fn valid(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
) -> bool {
    let node = &nodes[r.target];
    if !resource(&node.resource_type) {
        return true;
    }
    let endpoint = &nodes[source];
    if node.resource_type == "aws_cloudfront_multitenant_distribution"
        && super::service_evidence::known(evidence, r.target, "connection_mode")
            != Some("tenant-only")
    {
        return false;
    }
    if r.attribute == "distribution_id"
        && super::service_evidence::known(evidence, source, "connection_mode")
            != Some("tenant-only")
    {
        return false;
    }
    match r.attribute.as_str() {
        "customizations.web_acl.arn" | "web_acl_id" => {
            endpoint.waf_scope == Some(WafScope::CloudFront)
                && super::service_evidence::known(evidence, source, "scope") == Some("CLOUDFRONT")
        }
        "customizations.certificate.arn" | "viewer_certificate.acm_certificate_arn" => {
            super::scope::scope(endpoint)
                .and_then(|s| s.region)
                .is_some_and(|r| r.0 == "us-east-1")
        }
        "origin.domain_name" => evidence[r.target].0["origin"]
            .as_array()
            .is_some_and(|origins| {
                origins.iter().all(|o| {
                    o["domain_name"]
                        .as_str()
                        .is_some_and(|s| !s.contains("{{") && !s.contains("}}"))
                })
            }),
        _ => true,
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    !resource(&raw.nodes[r.target].resource_type)
        || r.sources.iter().all(|&s| {
            super::global_accelerator::compatible(
                raw.nodes[r.target].provider_configuration.as_ref(),
                raw.nodes[s].provider_configuration.as_ref(),
            )
        })
}
