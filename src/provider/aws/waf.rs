use super::service_references::scalar;
use crate::model::{Action, AttributeReference, TerraformEntity, WafScope};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/waf.rs"]
mod tests;

pub(crate) fn enrich(
    nodes: &mut [TerraformEntity],
    after: &[(&Value, &Value)],
    before: &[(&Value, &Value)],
) {
    for (index, node) in nodes.iter_mut().enumerate() {
        if node.provider.is_aws()
            && matches!(
                node.resource_type.as_str(),
                "aws_wafv2_web_acl" | "aws_wafv2_rule_group"
            )
        {
            let (value, unknown) = if node.action == Action::Delete || node.deposed_key.is_some() {
                before[index]
            } else {
                after[index]
            };
            node.waf_scope = match scalar(value, unknown, &["scope"]) {
                Some("CLOUDFRONT") => Some(WafScope::CloudFront),
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
