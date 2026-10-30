use super::service_evidence::{eligible, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_qualified.rs"]
mod tests;

fn supported(kind: &str) -> bool {
    matches!(
        kind,
        "aws_lambda_function_recursion_config"
            | "aws_lambda_function_scaling_config"
            | "aws_lambda_invocation"
            | "aws_lambda_provisioned_concurrency_config"
            | "aws_lambda_runtime_management_config"
    )
}

pub(super) fn label(kind: &str, attribute: &str) -> Option<&'static str> {
    if !supported(kind) {
        return None;
    }
    match attribute {
        "function_name" => Some("Configured Lambda function"),
        "qualifier" if kind != "aws_lambda_function_recursion_config" => Some("Function qualifier"),
        _ => None,
    }
}

fn unique<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    field: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = found.next()?;
    (found.next().is_none() && r.sources.len() == 1).then_some(r)
}

fn equal(evidence: &[(&Value, &Value)], r: &AttributeReference, field: &str) -> bool {
    r.source_attributes.len() == 1
        && r.source_attributes.contains(field)
        && known(evidence, r.target, &r.attribute)
            .is_some_and(|v| Some(v) == known(evidence, r.sources[0], field))
}

fn parent(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
) -> Option<usize> {
    let r = unique(refs, target, "function_name")?;
    if !eligible(nodes, r) {
        return None;
    }
    let source = r.sources[0];
    match nodes[source].resource_type.as_str() {
        "aws_lambda_function"
            if equal(evidence, r, "function_name") || equal(evidence, r, "arn") =>
        {
            Some(source)
        }
        "aws_lambda_alias" if equal(evidence, r, "function_name") => {
            let p = unique(refs, source, "function_name")?;
            (p.service_reference_verified
                && eligible(nodes, p)
                && (equal(evidence, p, "function_name") || equal(evidence, p, "arn"))
                && nodes[p.sources[0]].resource_type == "aws_lambda_function")
                .then_some(p.sources[0])
        }
        _ => None,
    }
}

fn version(value: &str) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|v| v > 0 && v.to_string() == value)
}

pub(super) fn bindings(raw: &crate::semantic::Input, reference: &AttributeReference) -> bool {
    ["function_name", "qualifier"].into_iter().all(|field| {
        raw.attributes
            .iter()
            .filter(|r| r.target == reference.target && r.attribute == field)
            .all(|r| {
                super::streaming::bindings(raw, r)
                    && r.sources.iter().all(|&s| {
                        raw.nodes[s].resource_type != "aws_lambda_alias"
                            || unique(&raw.attributes, s, "function_name").is_some_and(|p| {
                                p.service_reference_verified && super::streaming::bindings(raw, p)
                            })
                    })
            })
    })
}

fn qualifier(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    function: usize,
) -> bool {
    let kind = nodes[target].resource_type.as_str();
    if kind == "aws_lambda_function_recursion_config" {
        return true;
    }
    let (value, mask) = evidence[target];
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&mask["qualifier"], Value::Null | Value::Bool(false))
    {
        return false;
    }
    let selected = known(evidence, target, "qualifier");
    let special = match kind {
        "aws_lambda_function_scaling_config" => selected == Some("$LATEST.PUBLISHED"),
        "aws_lambda_invocation" | "aws_lambda_runtime_management_config" => {
            selected == Some("$LATEST") || value["qualifier"].is_null()
        }
        _ => false,
    };
    if special {
        return !refs.iter().any(|r| {
            r.target == target
                && r.attribute == "qualifier"
                && (r.has_references || !r.sources.is_empty() || !r.issues.is_empty())
        });
    }
    let Some(r) = unique(refs, target, "qualifier") else {
        return false;
    };
    if !eligible(nodes, r) {
        return false;
    }
    let source = r.sources[0];
    match nodes[source].resource_type.as_str() {
        "aws_lambda_function" => {
            source == function && equal(evidence, r, "version") && selected.is_some_and(version)
        }
        "aws_lambda_alias"
            if matches!(
                kind,
                "aws_lambda_invocation" | "aws_lambda_provisioned_concurrency_config"
            ) =>
        {
            equal(evidence, r, "name") && parent(nodes, refs, evidence, source) == Some(function)
        }
        _ => false,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
                return r.service_reference_verified;
            }
            parent(nodes, refs, evidence, r.target)
                .is_some_and(|function| qualifier(nodes, refs, evidence, r.target, function))
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        r.service_reference_verified = verified;
    }
}
