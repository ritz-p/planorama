use super::service_evidence::{eligible, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_async.rs"]
mod async_tests;
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
            | "aws_lambda_function_event_invoke_config"
            | "aws_lambda_permission"
    )
}

pub(super) fn label(kind: &str, attribute: &str) -> Option<&'static str> {
    if !supported(kind) {
        return None;
    }
    match attribute {
        "destination_config.on_success.destination"
            if kind == "aws_lambda_function_event_invoke_config" =>
        {
            Some("Async success destination")
        }
        "destination_config.on_failure.destination"
            if kind == "aws_lambda_function_event_invoke_config" =>
        {
            Some("Async failure destination")
        }
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

pub(super) fn parent(
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
    if nodes[target].resource_type == "aws_lambda_permission" {
        if nodes[source].resource_type == "aws_lambda_function"
            && equal(evidence, r, "qualified_arn")
            && function_arn(nodes, refs, evidence, source, "qualified_arn")
        {
            return Some(source);
        }
        if nodes[source].resource_type == "aws_lambda_alias"
            && equal(evidence, r, "arn")
            && function_arn(nodes, refs, evidence, source, "arn")
        {
            return parent(nodes, refs, evidence, source);
        }
    }
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
    super::streaming::bindings(raw, reference)
        && ["function_name", "qualifier", reference.attribute.as_str()]
            .into_iter()
            .all(|field| {
                raw.attributes
                    .iter()
                    .filter(|r| r.target == reference.target && r.attribute == field)
                    .all(|r| {
                        super::streaming::bindings(raw, r)
                            && r.sources.iter().all(|&s| {
                                raw.nodes[s].resource_type != "aws_lambda_alias"
                                    || unique(&raw.attributes, s, "function_name").is_some_and(
                                        |p| {
                                            p.service_reference_verified
                                                && super::streaming::bindings(raw, p)
                                        },
                                    )
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
    if kind == "aws_lambda_permission" {
        let Some(r) = unique(refs, target, "function_name") else {
            return false;
        };
        let qualified = r.source_attributes.contains("qualified_arn")
            || (nodes[r.sources[0]].resource_type == "aws_lambda_alias"
                && r.source_attributes.contains("arn"));
        if qualified {
            return value["qualifier"].is_null()
                && !refs
                    .iter()
                    .any(|r| r.target == target && r.attribute == "qualifier");
        }
    }
    let special = match kind {
        "aws_lambda_permission" => value["qualifier"].is_null(),
        "aws_lambda_function_scaling_config" => selected == Some("$LATEST.PUBLISHED"),
        "aws_lambda_invocation"
        | "aws_lambda_runtime_management_config"
        | "aws_lambda_function_event_invoke_config" => {
            selected == Some("$LATEST") || value["qualifier"].is_null()
        }
        _ => false,
    };
    if special {
        return !refs
            .iter()
            .any(|r| r.target == target && r.attribute == "qualifier");
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
                "aws_lambda_invocation"
                    | "aws_lambda_provisioned_concurrency_config"
                    | "aws_lambda_function_event_invoke_config"
                    | "aws_lambda_permission"
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
            parent(nodes, refs, evidence, r.target).is_some_and(|function| {
                qualifier(nodes, refs, evidence, r.target, function)
                    && (!r.attribute.starts_with("destination_config.")
                        || destination(nodes, refs, evidence, r))
            })
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        r.service_reference_verified = verified;
    }
}

fn destination(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if !eligible(nodes, r) || r.sources.len() != 1 || r.source_attributes.len() != 1 {
        return false;
    }
    let (value, mask) = evidence[r.target];
    let Some([block]) = value["destination_config"].as_array().map(Vec::as_slice) else {
        return false;
    };
    let branch = if r.attribute == "destination_config.on_success.destination" {
        "on_success"
    } else {
        "on_failure"
    };
    if block[branch].as_array().is_none_or(|v| v.len() != 1) {
        return false;
    }
    let Some(values) = super::streaming::values(value, mask, &r.attribute, false) else {
        return false;
    };
    let source = r.sources[0];
    let kind = nodes[source].resource_type.as_str();
    let field = if r.source_attributes.contains("qualified_arn") {
        "qualified_arn"
    } else {
        "arn"
    };
    if !r.source_attributes.contains(field)
        || values.len() != 1
        || known(evidence, source, field).is_none_or(|v| !values.contains(v))
    {
        return false;
    }
    match kind {
        "aws_sqs_queue" | "aws_sns_topic" => {
            let key = if kind == "aws_sqs_queue" {
                "fifo_queue"
            } else {
                "fifo_topic"
            };
            let (v, m) = evidence[source];
            field == "arn"
                && v[key] == Value::Bool(false)
                && matches!(m, Value::Null | Value::Bool(false) | Value::Object(_))
                && matches!(&m[key], Value::Null | Value::Bool(false))
        }
        "aws_cloudwatch_event_bus" => field == "arn",
        "aws_s3_bucket" => field == "arn" && branch == "on_failure",
        "aws_lambda_function" | "aws_lambda_alias" => {
            function_arn(nodes, refs, evidence, source, field)
        }
        _ => false,
    }
}

pub(super) fn function_arn(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    source: usize,
    field: &str,
) -> bool {
    match nodes[source].resource_type.as_str() {
        "aws_lambda_function" => {
            field == "arn"
                || (field == "qualified_arn"
                    && known(evidence, source, "version")
                        .filter(|v| version(v))
                        .zip(known(evidence, source, "arn"))
                        .is_some_and(|(v, arn)| {
                            known(evidence, source, "qualified_arn")
                                == Some(format!("{arn}:{v}").as_str())
                        }))
        }
        "aws_lambda_alias" => {
            field == "arn"
                && parent(nodes, refs, evidence, source)
                    .and_then(|p| known(evidence, p, "arn"))
                    .zip(known(evidence, source, "name"))
                    .is_some_and(|(arn, name)| {
                        known(evidence, source, "arn") == Some(format!("{arn}:{name}").as_str())
                    })
        }
        _ => false,
    }
}
