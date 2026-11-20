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
                    if historical || effective_region(node).is_none_or(|r| r.0 == "us-east-1") =>
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
        | (
            "aws_wafv2_web_acl",
            "rule.statement.rule_group_reference_statement.arn",
            "aws_wafv2_rule_group",
        ) => &["arn"],
        _ => &[],
    }
}

pub(super) fn compatible(left: &TerraformEntity, right: &TerraformEntity, regional: bool) -> bool {
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

pub(super) fn verify_scope(nodes: &[TerraformEntity], reference: &AttributeReference) -> bool {
    let [source] = reference.sources.as_slice() else {
        return false;
    };
    let node = &nodes[reference.target];
    let endpoint = &nodes[*source];
    match (node.resource_type.as_str(), reference.attribute.as_str()) {
        ("aws_cloudfront_distribution", "web_acl_id") => {
            endpoint.waf_scope == Some(WafScope::CloudFront)
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

pub(crate) fn inherit_scope(
    nodes: &mut [TerraformEntity],
    refs: &[AttributeReference],
    values: &[(&Value, &Value)],
) {
    for i in 0..nodes.len() {
        let field = match nodes[i].resource_type.as_str() {
            "aws_wafv2_web_acl_logging_configuration" => "resource_arn",
            "aws_wafv2_web_acl_rule" | "aws_wafv2_web_acl_rule_group_association" => "web_acl_arn",
            _ => continue,
        };
        nodes[i].waf_scope = parent_scope(nodes, refs, values, i, field);
    }
}

fn parent_scope(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    values: &[(&Value, &Value)],
    i: usize,
    field: &str,
) -> Option<WafScope> {
    let r = super::lambda_core::unique(refs, i, field)?;
    let [source] = *r.sources.as_slice() else {
        return None;
    };
    let node = &nodes[i];
    let acl = &nodes[source];
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == crate::model::EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    if !current(node)
        || !current(acl)
        || acl.resource_type != "aws_wafv2_web_acl"
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains("arn")
        || !super::configured_links::same_account(
            node.provider_configuration.as_ref(),
            acl.provider_configuration.as_ref(),
        )
    {
        return None;
    }
    let arn = super::service_evidence::known(values, i, field)?;
    if super::service_evidence::known(values, source, "arn") != Some(arn) {
        return None;
    }
    let region = effective_region(node);
    let scope = acl.waf_scope?;
    match scope {
        WafScope::Regional if super::scope(acl)?.region.as_ref() == Some(region?) => Some(scope),
        WafScope::CloudFront if region.is_none_or(|r| r.0 == "us-east-1") => Some(scope),
        _ => None,
    }
}

pub(super) fn protected_fields(kind: &str) -> &'static [&'static str] {
    match super::aliases::canonical(kind) {
        "aws_lb"
        | "aws_api_gateway_stage"
        | "aws_cognito_user_pool"
        | "aws_appsync_graphql_api"
        | "aws_apprunner_service"
        | "aws_amplify_app" => &["arn"],
        "aws_bedrockagentcore_gateway" => &["gateway_arn"],
        "aws_verifiedaccess_instance" => &["id"],
        _ => &[],
    }
}
pub(super) fn annotate_associations(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let node = &nodes[r.target];
            if node.resource_type != "aws_wafv2_web_acl_association"
                || !matches!(r.attribute.as_str(), "web_acl_arn" | "resource_arn")
            {
                return None;
            }
            let valid = if r.attribute == "web_acl_arn" {
                let protected = super::lambda_core::unique(refs, r.target, "resource_arn");
                let scope = match protected.map(|r| r.sources.as_slice()) {
                    Some([s]) if nodes[*s].resource_type == "aws_amplify_app" => {
                        WafScope::CloudFront
                    }
                    _ => WafScope::Regional,
                };
                parent_scope(nodes, refs, values, r.target, "web_acl_arn") == Some(scope)
            } else {
                association_endpoint(nodes, r, values)
            };
            Some((i, valid))
        })
        .collect();
    for (i, valid) in verified {
        refs[i].service_reference_verified = valid;
    }
}
fn association_endpoint(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    values: &[(&Value, &Value)],
) -> bool {
    let [s] = *r.sources.as_slice() else {
        return false;
    };
    let node = &nodes[r.target];
    let endpoint = &nodes[s];
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == crate::model::EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    let Some(actual) = super::service_evidence::known(values, r.target, &r.attribute) else {
        return false;
    };
    let Some(field) = r.source_attributes.iter().next() else {
        return false;
    };
    let kind = super::aliases::entity_type(endpoint);
    let Some(region) = effective_region(node) else {
        return false;
    };
    let region_matches = if kind == "aws_amplify_app" {
        region.0 == "us-east-1"
    } else {
        effective_region(endpoint) == Some(region)
    };
    if !current(node)
        || !current(endpoint)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.source_attributes.len() != 1
        || !region_matches
        || !super::configured_links::same_account(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
        )
    {
        return false;
    }
    let Some(output) = super::service_evidence::known(values, s, field) else {
        return false;
    };
    if !protected_fields(kind).contains(&field.as_str()) {
        return false;
    }
    if kind == "aws_verifiedaccess_instance" {
        let Some(account) = endpoint
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
        else {
            return false;
        };
        let Some(region) = effective_region(endpoint) else {
            return false;
        };
        let partition = if region.0.starts_with("cn-") {
            "aws-cn"
        } else if region.0.starts_with("us-gov-") {
            "aws-us-gov"
        } else {
            "aws"
        };
        return actual
            == format!(
                "arn:{partition}:ec2:{}:{}:verified-access-instance/{output}",
                region.0,
                account.as_str()
            );
    }
    if actual != output {
        return false;
    }
    match kind {
        "aws_lb" => {
            super::service_evidence::known(values, s, "load_balancer_type") == Some("application")
        }
        "aws_api_gateway_stage" => true,
        "aws_cognito_user_pool" => regional_arn(endpoint, actual, "cognito-idp", "userpool/"),
        "aws_appsync_graphql_api" => regional_arn(endpoint, actual, "appsync", "apis/"),
        "aws_apprunner_service" => regional_arn(endpoint, actual, "apprunner", "service/"),
        "aws_amplify_app" => regional_arn(endpoint, actual, "amplify", "apps/"),
        "aws_bedrockagentcore_gateway" => {
            regional_arn(endpoint, actual, "bedrock-agentcore", "gateway/")
        }
        _ => false,
    }
}

fn regional_arn(endpoint: &TerraformEntity, arn: &str, service: &str, prefix: &str) -> bool {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, actual, region, account, resource] = parts.as_slice() else {
        return false;
    };
    !partition.is_empty()
        && *actual == service
        && effective_region(endpoint).is_some_and(|r| r.0 == *region)
        && crate::model::AwsAccountScope::new(account).is_some_and(|a| {
            endpoint
                .provider_configuration
                .as_ref()
                .and_then(|p| p.account.as_ref())
                .is_none_or(|bound| bound == &a)
        })
        && resource
            .strip_prefix(prefix)
            .is_some_and(|id| !id.is_empty())
}
