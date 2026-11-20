use super::service_references::scalar;
use crate::model::{Action, AttributeReference, TerraformEntity, WafScope};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/waf.rs"]
mod tests;

pub(super) fn explicit_scope(kind: &str) -> bool {
    matches!(
        kind,
        "aws_wafv2_web_acl"
            | "aws_wafv2_rule_group"
            | "aws_wafv2_ip_set"
            | "aws_wafv2_regex_pattern_set"
            | "aws_wafv2_api_key"
    )
}

pub(crate) fn enrich(
    nodes: &mut [TerraformEntity],
    after: &[(&Value, &Value)],
    before: &[(&Value, &Value)],
    prior_scope_hidden: impl Fn(&TerraformEntity) -> bool,
) {
    for (index, node) in nodes.iter_mut().enumerate() {
        if node.provider.is_aws()
            && node.mode == crate::model::EntityMode::Managed
            && explicit_scope(&node.resource_type)
        {
            let historical = node.action == Action::Delete || node.deposed_key.is_some();
            let (value, unknown) = if historical {
                if prior_scope_hidden(node) {
                    continue;
                }
                before[index]
            } else {
                after[index]
            };
            node.waf_scope = match super::nested_references::value(value, unknown, "scope")
                .and_then(Value::as_str)
            {
                Some("CLOUDFRONT")
                    if historical || effective_region(node).is_some_and(|r| r.0 == "us-east-1") =>
                {
                    Some(WafScope::CloudFront)
                }
                Some("REGIONAL") => Some(WafScope::Regional),
                _ => None,
            };
        }
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_cloudfront_distribution", "web_acl_id")
        | ("aws_wafv2_web_acl_association", "web_acl_arn") => Some("Web ACL reference"),
        ("aws_wafv2_web_acl_association", "resource_arn") => Some("Protected resource reference"),
        ("aws_wafv2_web_acl", "rule.statement.rule_group_reference_statement.arn") => {
            Some("Rule group reference")
        }
        _ => None,
    }
}

pub(super) fn source_fields(
    resource: &str,
    attribute: &str,
    source: &str,
) -> &'static [&'static str] {
    match (resource, attribute, source) {
        ("aws_cloudfront_distribution", "web_acl_id", "aws_wafv2_web_acl")
        | ("aws_wafv2_web_acl_association", "web_acl_arn", "aws_wafv2_web_acl")
        | ("aws_wafv2_web_acl_association", "resource_arn", "aws_lb" | "aws_api_gateway_stage")
        | (
            "aws_wafv2_web_acl",
            "rule.statement.rule_group_reference_statement.arn",
            "aws_wafv2_rule_group",
        ) => &["arn"],
        _ => &[],
    }
}

fn compatible(left: &TerraformEntity, right: &TerraformEntity, regional: bool) -> bool {
    let (Some(left_scope), Some(right_scope)) = (super::scope(left), super::scope(right)) else {
        return false;
    };
    if regional && (left_scope.region.is_none() || left_scope.region != right_scope.region) {
        return false;
    }
    if left
        .provider_configuration
        .as_ref()
        .is_some_and(|c| c.account_conflict)
        || right
            .provider_configuration
            .as_ref()
            .is_some_and(|c| c.account_conflict)
    {
        return false;
    }
    match (&left_scope.account, &right_scope.account) {
        (Some(a), Some(b)) => a == b,
        (None, None) => {
            left.provider_configuration.is_some()
                && left.provider_configuration == right.provider_configuration
        }
        _ => false,
    }
}

pub(super) fn verify_scope(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let [source] = reference.sources.as_slice() else {
        return false;
    };
    let node = &nodes[reference.target];
    let endpoint = &nodes[*source];
    match (node.resource_type.as_str(), reference.attribute.as_str()) {
        ("aws_cloudfront_distribution", "web_acl_id") => {
            endpoint.waf_scope == Some(WafScope::CloudFront)
        }
        ("aws_wafv2_web_acl_association", "web_acl_arn") => {
            endpoint.waf_scope == Some(WafScope::Regional) && compatible(node, endpoint, true)
        }
        ("aws_wafv2_web_acl_association", "resource_arn") => {
            compatible(node, endpoint, true)
                && (endpoint.resource_type == "aws_api_gateway_stage"
                    || scalar(
                        evidence[*source].0,
                        evidence[*source].1,
                        &["load_balancer_type"],
                    ) == Some("application"))
        }
        ("aws_wafv2_web_acl", "rule.statement.rule_group_reference_statement.arn") => {
            node.waf_scope.is_some()
                && node.waf_scope == endpoint.waf_scope
                && compatible(node, endpoint, node.waf_scope == Some(WafScope::Regional))
        }
        _ => true,
    }
}

pub(super) fn effective_region(node: &TerraformEntity) -> Option<&crate::model::RegionScope> {
    if node.has_region_override {
        node.region_override.as_ref()
    } else {
        node.provider_configuration.as_ref()?.region.as_ref()
    }
}
