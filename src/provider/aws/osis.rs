use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/osis.rs"]
mod tests;

const SUBNETS: &str = "vpc_options.subnet_ids";
const GROUPS: &str = "vpc_options.security_group_ids";

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (kind, path) {
        ("aws_osis_pipeline" | "aws_osis_pipeline_endpoint", SUBNETS) => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_osis_pipeline" | "aws_osis_pipeline_endpoint", GROUPS) => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_osis_pipeline", "pipeline_role_arn") => {
            Some(("aws_iam_role", "arn", false, "Pipeline role"))
        }
        ("aws_osis_pipeline", "encryption_at_rest_options.kms_key_arn") => {
            Some(("aws_kms_key", "arn", false, "Buffer encryption key"))
        }
        ("aws_osis_pipeline", "log_publishing_options.cloudwatch_log_destination.log_group") => {
            Some((
                "aws_cloudwatch_log_group",
                "name",
                false,
                "Pipeline log group",
            ))
        }
        ("aws_osis_pipeline_endpoint", "pipeline_arn")
        | ("aws_osis_resource_policy", "resource_arn") => Some((
            "aws_osis_pipeline",
            "pipeline_arn",
            false,
            "Ingestion pipeline",
        )),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
}

fn field<'a>(
    mut value: &'a Value,
    mut mask: &'a Value,
    path: &str,
) -> Option<(&'a Value, &'a Value)> {
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        if !value.is_object()
            || !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        value = value.get(part)?;
        mask = &mask[part];
        if parts.peek().is_some() {
            let [single] = value.as_array()?.as_slice() else {
                return None;
            };
            value = single;
            mask = match mask {
                Value::Null | Value::Bool(false) => &Value::Null,
                Value::Array(m) if m.len() == 1 => &m[0],
                _ => return None,
            };
        }
    }
    Some((value, mask))
}

fn known<'a>(value: &'a Value, mask: &'a Value, path: &str) -> Option<&'a Value> {
    let (value, mask) = field(value, mask, path)?;
    matches!(mask, Value::Null | Value::Bool(false)).then_some(value)
}

fn arn(value: &str, service: &str, prefix: &str) -> bool {
    let parts: Vec<_> = value.splitn(6, ':').collect();
    parts.len() == 6
        && parts[0] == "arn"
        && parts[1].starts_with("aws")
        && parts[2] == service
        && !parts[3].is_empty()
        && parts[4].len() == 12
        && parts[4].bytes().all(|c| c.is_ascii_digit())
        && parts[5]
            .strip_prefix(prefix)
            .is_some_and(|s| !s.is_empty() && !s.contains(['*', '?']))
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let Some((kind, output, collection, _)) = rule(&nodes[r.target].resource_type, &r.attribute)
    else {
        return false;
    };
    if super::lambda_core::unique(refs, r.target, &r.attribute).is_none()
        || !super::service_evidence::eligible_with_global_role(nodes, r, kind == "aws_iam_role")
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(output)
    {
        return false;
    }
    let (value, mask) = evidence[r.target];
    if field(value, mask, &r.attribute).is_none() {
        return false;
    }
    if output == "name"
        && known(value, mask, "log_publishing_options.is_logging_enabled")
            != Some(&Value::Bool(true))
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
        return false;
    };
    if kind == "aws_osis_pipeline" && !actual.iter().all(|a| arn(a, "osis", "pipeline/")) {
        return false;
    }
    if kind == "aws_kms_key" && !actual.iter().all(|a| arn(a, "kms", "key/")) {
        return false;
    }
    let expected = r
        .sources
        .iter()
        .map(|&s| {
            (nodes[s].resource_type == kind)
                .then(|| super::service_evidence::known(evidence, s, output))
                .flatten()
        })
        .collect::<Option<BTreeSet<_>>>();
    expected.is_some_and(|expected| expected.len() == r.sources.len() && expected == actual)
}

fn network_context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<Vec<usize>> {
    let (value, mask) = evidence[r.target];
    if nodes[r.target].resource_type == "aws_osis_pipeline"
        && known(value, mask, "vpc_options.vpc_endpoint_management").and_then(Value::as_str)
            != Some("SERVICE")
    {
        return None;
    }
    let parents = r
        .sources
        .iter()
        .map(|&s| super::lambda_core::parent(nodes, refs, evidence, s))
        .collect::<Option<BTreeSet<_>>>()?;
    if parents.len() != 1 {
        return None;
    }
    let parent = *parents.first()?;
    if nodes[r.target].resource_type == "aws_osis_pipeline_endpoint" {
        if !matches!(&mask["vpc_id"], Value::Null | Value::Bool(false)) {
            return None;
        }
        if !value["vpc_id"].is_null()
            && super::service_evidence::known(evidence, r.target, "vpc_id")
                != super::service_evidence::known(evidence, parent, "id")
        {
            return None;
        }
    }
    Some(vec![parent])
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let (_, _, collection, _) = rule(&nodes[r.target].resource_type, &r.attribute)?;
            let valid = verify(nodes, refs, r, evidence);
            let context = if valid && collection {
                network_context(nodes, refs, r, evidence).unwrap_or_default()
            } else {
                Vec::new()
            };
            Some((valid, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&source| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[source];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            r.attribute == "pipeline_role_arn",
        )
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for kind in ["aws_osis_pipeline", "aws_osis_pipeline_endpoint"] {
        super::lambda_core::present_network(raw, graph, kind, (SUBNETS, GROUPS), bindings);
    }
}
