use super::service_evidence::{eligible, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_policy.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_lambda_resource_policy", "resource_arn") => Some("Lambda policy resource"),
        ("aws_lambda_permission", "source_arn") => Some("Permission source"),
        _ => None,
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    super::streaming::bindings(raw, r)
        && r.sources.iter().all(|&source| {
            raw.nodes[source].resource_type != "aws_lambda_alias"
                || raw
                    .attributes
                    .iter()
                    .find(|p| p.target == source && p.attribute == "function_name")
                    .is_some_and(|p| {
                        p.service_reference_verified && super::streaming::bindings(raw, p)
                    })
        })
}

fn source(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let source = r.sources[0];
    let Some(principal) = known(evidence, r.target, "principal") else {
        return false;
    };
    let kind = nodes[source].resource_type.as_str();
    if kind == "aws_lb_target_group" && known(evidence, source, "target_type") != Some("lambda") {
        return false;
    }
    let (field, service) = match kind {
        "aws_s3_bucket" => ("arn", "s3.amazonaws.com"),
        "aws_sns_topic" => ("arn", "sns.amazonaws.com"),
        "aws_cloudwatch_event_rule" => ("arn", "events.amazonaws.com"),
        "aws_api_gateway_rest_api" | "aws_apigatewayv2_api" => {
            ("execution_arn", "apigateway.amazonaws.com")
        }
        "aws_cloudwatch_log_group" => ("arn", "logs"),
        "aws_cognito_user_pool" => ("arn", "cognito-idp.amazonaws.com"),
        "aws_lb_target_group" => ("arn", "elasticloadbalancing.amazonaws.com"),
        _ => return false,
    };
    if !r.source_attributes.contains(field) {
        return false;
    }
    let (Some(actual), Some(arn)) = (
        known(evidence, r.target, "source_arn"),
        known(evidence, source, field),
    ) else {
        return false;
    };
    let principal_ok = if service == "logs" {
        super::scope::scope(&nodes[source])
            .and_then(|s| s.region)
            .is_some_and(|region| principal == format!("logs.{}.amazonaws.com", region.0))
    } else {
        principal == service
    };
    if !principal_ok {
        return false;
    }
    let (v, m) = evidence[r.target];
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&m["source_account"], Value::Null | Value::Bool(false))
    {
        return false;
    }
    if !v["source_account"].is_null() {
        let Some(account) = known(evidence, r.target, "source_account") else {
            return false;
        };
        let source_account = if kind == "aws_s3_bucket" {
            nodes[source]
                .provider_configuration
                .as_ref()
                .and_then(|p| p.account.as_ref())
                .map(|a| a.as_str())
        } else {
            arn.split(':').nth(4)
        };
        if source_account != Some(account) {
            return false;
        }
    }
    if actual == arn {
        return true;
    }
    if kind == "aws_cloudwatch_log_group" {
        return actual == format!("{arn}:*");
    }
    field == "execution_arn"
        && actual
            .strip_prefix(arn)
            .and_then(|suffix| suffix.strip_prefix('/'))
            .is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"/*_-.".contains(&c))
            })
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
                return r.service_reference_verified;
            }
            if !eligible(nodes, r) || r.sources.len() != 1 || r.source_attributes.len() != 1 {
                return false;
            }
            if r.attribute == "source_arn" {
                return source(nodes, evidence, r);
            }
            let source = r.sources[0];
            let field = if r.source_attributes.contains("qualified_arn") {
                "qualified_arn"
            } else {
                "arn"
            };
            r.source_attributes.contains(field)
                && known(evidence, r.target, "resource_arn")
                    .is_some_and(|v| Some(v) == known(evidence, source, field))
                && super::lambda_qualified::function_arn(nodes, refs, evidence, source, field)
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(verified) {
        r.service_reference_verified = verified;
    }
}
