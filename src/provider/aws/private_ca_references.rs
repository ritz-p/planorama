use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/private_ca_references.rs"]
mod tests;

const CRL: &str = "revocation_configuration.crl_configuration.s3_bucket_name";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_acmpca_certificate_authority" | "aws_acmpca_permission" | "aws_acmpca_policy"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_acmpca_certificate_authority", CRL) => Some("CRL bucket"),
        ("aws_acmpca_permission", "certificate_authority_arn") => Some("Permission CA"),
        ("aws_acmpca_policy", "resource_arn") => Some("Policy CA"),
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == "aws_acmpca_certificate_authority"
        && path == "revocation_configuration.0.crl_configuration.0.s3_bucket_name"
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_acmpca_certificate_authority", CRL, "aws_s3_bucket") => &["id", "bucket"],
        (
            "aws_acmpca_permission",
            "certificate_authority_arn",
            "aws_acmpca_certificate_authority",
        )
        | ("aws_acmpca_policy", "resource_arn", "aws_acmpca_certificate_authority") => &["arn"],
        _ => &[],
    }
}
fn shape(value: &Value, path: &str) -> bool {
    if path != CRL {
        return value[path].is_string();
    }
    value["revocation_configuration"]
        .as_array()
        .is_some_and(|a| a.len() == 1)
        && value["revocation_configuration"][0]["crl_configuration"]
            .as_array()
            .is_some_and(|a| a.len() == 1)
        && value["revocation_configuration"][0]["crl_configuration"][0]["s3_bucket_name"]
            .is_string()
}
fn compatible(nodes: &[TerraformEntity], t: usize, s: usize) -> bool {
    let consistent = |node: &TerraformEntity| {
        let local = if node.module == "root" {
            Some(node.address.as_str())
        } else {
            node.address
                .strip_prefix(&node.module)
                .and_then(|s| s.strip_prefix('.'))
        };
        local
            .and_then(|s| s.split_once('.'))
            .is_some_and(|(kind, _)| kind == node.resource_type)
    };
    if !consistent(&nodes[t]) || !consistent(&nodes[s]) {
        return false;
    }

    let l = super::scope::scope(&nodes[t]);
    let r = super::scope::scope(&nodes[s]);
    l.as_ref().and_then(|s| s.region.as_ref()).is_some()
        && r.as_ref().and_then(|s| s.region.as_ref()).is_some()
        && super::eventbridge::bindings(
            nodes[t].provider_configuration.as_ref(),
            nodes[s].provider_configuration.as_ref(),
            l.as_ref().and_then(|s| s.region.as_ref()),
            r.as_ref().and_then(|s| s.region.as_ref()),
            false,
        )
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
            let (value, mask) = evidence[r.target];
            let enabled = r.attribute != CRL
                || super::nested_references::value(
                    value,
                    mask,
                    "revocation_configuration.0.crl_configuration.0.enabled",
                ) == Some(&Value::Bool(true));
            Some(
                enabled
                    && shape(value, &r.attribute)
                    && super::nested_references::verify(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |s, _| compatible(nodes, r.target, s),
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
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
    r.sources.iter().all(|&s| {
        let l = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            false,
        )
    })
}
