use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/synthetics.rs"]
mod tests;
const CANARY: &str = "aws_synthetics_canary";
const SUBNETS: &str = "vpc_config.subnet_ids";
const GROUPS: &str = "vpc_config.security_group_ids";
const ARTIFACT: &str = "artifact_config.s3_encryption.kms_key_arn";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, CANARY | "aws_synthetics_group_association")
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (CANARY, "execution_role_arn") => Some("Canary execution role"),
        (CANARY, "s3_bucket") => Some("Canary script bucket"),
        (CANARY, "kms_key_arn") => Some("Canary environment key"),
        (CANARY, ARTIFACT) => Some("Canary artifact key"),
        (CANARY, SUBNETS) => Some("Canary subnets"),
        (CANARY, GROUPS) => Some("Canary security groups"),
        ("aws_synthetics_group_association", "group_name") => Some("Synthetics group"),
        ("aws_synthetics_group_association", "canary_arn") => Some("Associated canary"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("execution_role_arn", "aws_iam_role") => &["arn"],
        ("s3_bucket", "aws_s3_bucket") => &["bucket", "id"],
        ("kms_key_arn" | ARTIFACT, kind) if super::kms::key_outputs(kind).contains(&"arn") => {
            &["arn"]
        }
        (SUBNETS, "aws_subnet") | (GROUPS, "aws_security_group") => &["id"],
        ("group_name", "aws_synthetics_group") => &["name"],
        ("canary_arn", CANARY) => &["arn"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}
fn singleton(value: &Value) -> bool {
    value.as_array().is_some_and(|a| a.len() == 1)
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let (v, m) = e[r.target];
            let network = matches!(r.attribute.as_str(), SUBNETS | GROUPS);
            let shape = if network {
                singleton(&v["vpc_config"])
            } else if r.attribute == ARTIFACT {
                singleton(&v["artifact_config"])
                    && singleton(&v["artifact_config"][0]["s3_encryption"])
                    && super::nested_references::value(
                        v,
                        m,
                        "artifact_config.0.s3_encryption.0.encryption_mode",
                    )
                    .and_then(Value::as_str)
                        == Some("SSE_KMS")
            } else {
                true
            };
            let verified = shape
                && super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                    super::nested_references::value(v, m, path)
                        .is_some_and(|v| if network { v.is_array() } else { v.is_string() })
                })
                .is_some();
            let context = if verified && network {
                r.sources
                    .iter()
                    .map(|&s| super::lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<BTreeSet<_>>>()
                    .filter(|parents| parents.len() == 1)
                    .map(|p| p.into_iter().collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Some((verified, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let a = &raw.nodes[r.target];
            let b = &raw.nodes[s];
            let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            super::iam_policies::same_account(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
            ) && ar.is_some()
                && (b.resource_type == "aws_iam_role" || ar == br)
        })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(raw, graph, CANARY, (SUBNETS, GROUPS), bindings);
}
