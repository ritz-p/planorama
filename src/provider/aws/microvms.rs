use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/microvms.rs"]
mod tests;

const GROUP: &str = "logging.cloudwatch.log_group";
const STREAM: &str = "logging.cloudwatch.log_stream";

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (kind, path) {
        ("aws_lambdamicrovms_image", "base_image_arn") => {
            Some(("aws_lambdamicrovms_image", "arn", false, "Base image"))
        }
        ("aws_lambdamicrovms_image", "build_role_arn") => {
            Some(("aws_iam_role", "arn", false, "Image build role"))
        }
        ("aws_lambdamicrovms_image", "egress_network_connectors") => Some((
            "aws_lambdacore_network_connector",
            "arn",
            true,
            "Image egress connectors",
        )),
        ("aws_lambdamicrovms_microvm", "image_arn") => {
            Some(("aws_lambdamicrovms_image", "arn", false, "MicroVM image"))
        }
        ("aws_lambdamicrovms_microvm", "execution_role_arn") => {
            Some(("aws_iam_role", "arn", false, "MicroVM execution role"))
        }
        ("aws_lambdamicrovms_microvm", "ingress_network_connectors") => Some((
            "aws_lambdacore_network_connector",
            "arn",
            true,
            "MicroVM ingress connectors",
        )),
        ("aws_lambdamicrovms_microvm", GROUP) => Some((
            "aws_cloudwatch_log_group",
            "name",
            false,
            "MicroVM log group",
        )),
        ("aws_lambdamicrovms_microvm", STREAM) => Some((
            "aws_cloudwatch_log_stream",
            "name",
            false,
            "MicroVM log stream",
        )),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
}

fn proof(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    let Some((kind, field, collection, _)) = rule(&nodes[r.target].resource_type, &r.attribute)
    else {
        return false;
    };
    if !super::service_evidence::eligible_with_global_role(nodes, r, kind == "aws_iam_role")
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(field)
        || r.sources.contains(&r.target)
    {
        return false;
    }
    let (value, mask) = evidence[r.target];
    if matches!(r.attribute.as_str(), GROUP | STREAM)
        && (value["logging"].as_array().is_none_or(|v| v.len() != 1)
            || value["logging"][0]["cloudwatch"]
                .as_array()
                .is_none_or(|v| v.len() != 1)
            || !(value["logging"][0]["disabled"].is_null()
                || value["logging"][0]["disabled"]
                    .as_array()
                    .is_some_and(Vec::is_empty))
            || !matches!(
                mask["logging"][0]["disabled"],
                Value::Null | Value::Bool(false)
            ))
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
        return false;
    };
    let expected = r
        .sources
        .iter()
        .map(|&s| {
            (nodes[s].resource_type == kind)
                .then(|| super::service_evidence::known(evidence, s, field))
                .flatten()
        })
        .collect::<Option<BTreeSet<_>>>();
    if expected.as_ref() != Some(&actual) || actual.len() != r.sources.len() {
        return false;
    }
    if kind == "aws_lambdamicrovms_image" {
        let version = if r.attribute == "base_image_arn" {
            "base_image_version"
        } else {
            "image_version"
        };
        return super::service_evidence::known(evidence, r.target, version).is_some_and(|v| {
            super::service_evidence::known(evidence, r.sources[0], "image_version") == Some(v)
        });
    }
    true
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let valid = super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                && proof(nodes, r, evidence);
            if !valid || r.attribute != STREAM {
                return Some((valid, Vec::new()));
            }
            let group = super::lambda_core::unique(refs, r.target, GROUP)
                .filter(|g| proof(nodes, g, evidence));
            let parent = super::lambda_core::unique(refs, r.sources[0], "log_group_name");
            let context = group
                .zip(parent)
                .filter(|(g, p)| {
                    super::service_evidence::eligible(nodes, p)
                        && p.sources == g.sources
                        && p.source_attributes.len() == 1
                        && p.source_attributes.contains("name")
                        && super::service_evidence::known(evidence, p.target, "log_group_name")
                            .is_some_and(|name| {
                                super::service_evidence::known(evidence, g.sources[0], "name")
                                    == Some(name)
                            })
                })
                .map(|(g, _)| g.sources.clone())
                .unwrap_or_default();
            Some((!context.is_empty(), context))
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
    let compatible = |r: &AttributeReference| {
        r.sources.iter().all(|&s| {
            let left = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                left.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                matches!(
                    r.attribute.as_str(),
                    "build_role_arn" | "execution_role_arn"
                ),
            )
        })
    };
    compatible(r)
        && (r.attribute != STREAM
            || (super::lambda_core::unique(&raw.attributes, r.target, GROUP)
                .is_some_and(compatible)
                && r.sources
                    .first()
                    .and_then(|&s| super::lambda_core::unique(&raw.attributes, s, "log_group_name"))
                    .is_some_and(compatible)))
}
