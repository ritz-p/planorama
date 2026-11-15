use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/glue_network.rs"]
mod tests;

fn network(kind: &str) -> Option<(&'static str, &'static str)> {
    match kind {
        "aws_glue_connection" => Some((
            "physical_connection_requirements.subnet_id",
            "physical_connection_requirements.security_group_id_list",
        )),
        "aws_glue_dev_endpoint" => Some(("subnet_id", "security_group_ids")),
        _ => None,
    }
}
fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    let (subnet, groups) = network(kind)?;
    if path == subnet {
        return Some(("aws_subnet", "id", false, "Configured subnet"));
    }
    if path == groups {
        return Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        ));
    }
    match (kind, path) {
        ("aws_glue_connection", "authentication_configuration.kms_key_arn") => {
            Some(("aws_kms_key", "arn", false, "Authentication key"))
        }
        ("aws_glue_connection", "authentication_configuration.secret_arn") => Some((
            "aws_secretsmanager_secret",
            "arn",
            false,
            "Authentication secret",
        )),
        ("aws_glue_dev_endpoint", "role_arn") => {
            Some(("aws_iam_role", "arn", false, "Development role"))
        }
        ("aws_glue_dev_endpoint", "security_configuration") => Some((
            "aws_glue_security_configuration",
            "name",
            false,
            "Security configuration",
        )),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
}
fn known<'a>(value: &'a Value, mask: &Value, path: &str) -> Option<&'a Value> {
    if let Some((block, leaf)) = path.split_once('.') {
        if value[block].as_array().is_none_or(|a| a.len() != 1) {
            return None;
        }
        super::nested_references::value(value, mask, &format!("{block}.0.{leaf}"))
    } else {
        super::nested_references::value(value, mask, path)
    }
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
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
    if known(value, mask, &r.attribute).is_none() {
        return false;
    }
    if r.attribute.starts_with("authentication_configuration.")
        && !matches!(
            known(
                value,
                mask,
                "authentication_configuration.authentication_type"
            )
            .and_then(Value::as_str),
            Some("BASIC" | "CUSTOM" | "IAM" | "OAUTH2")
        )
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
        return false;
    };
    let expected = r
        .sources
        .iter()
        .map(|&source| {
            let source_type = nodes[source].resource_type.as_str();
            let correct = if kind == "aws_kms_key" {
                super::kms::key_outputs(source_type).contains(&"arn")
            } else {
                source_type == kind
            };
            correct
                .then(|| super::service_evidence::known(evidence, source, output))
                .flatten()
        })
        .collect::<Option<BTreeSet<_>>>();
    expected.is_some_and(|expected| expected.len() == r.sources.len() && expected == actual)
}
fn context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    let kind = nodes[r.target].resource_type.as_str();
    let (subnet, groups) = network(kind)?;
    if r.attribute != subnet && r.attribute != groups {
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
    let (value, mask) = evidence[r.target];
    if kind == "aws_glue_connection" && r.attribute == subnet {
        let zone = known(
            value,
            mask,
            "physical_connection_requirements.availability_zone",
        )?;
        if !zone.is_null()
            && !zone.as_str().filter(|s| !s.is_empty()).is_some_and(|zone| {
                super::service_evidence::known(evidence, r.sources[0], "availability_zone")
                    == Some(zone)
            })
        {
            return None;
        }
    }
    if kind == "aws_glue_dev_endpoint" {
        let vpc = known(value, mask, "vpc_id")?;
        if !vpc.is_null() && vpc.as_str() != super::service_evidence::known(evidence, parent, "id")
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
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let verified = verify(nodes, refs, evidence, r);
            let context = if verified {
                context(nodes, refs, evidence, r).unwrap_or_default()
            } else {
                Vec::new()
            };
            Some((verified, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((verified, context)) = result {
            r.service_reference_verified = verified;
            r.endpoint_network_context = context;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            r.attribute == "role_arn",
        )
    })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for kind in ["aws_glue_connection", "aws_glue_dev_endpoint"] {
        super::lambda_core::present_network(raw, graph, kind, network(kind).unwrap(), bindings);
    }
}
