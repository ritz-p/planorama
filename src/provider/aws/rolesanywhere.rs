use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/rolesanywhere.rs"]
mod tests;

const CA: &str = "source.source_data.acm_pca_arn";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_rolesanywhere_profile" | "aws_rolesanywhere_trust_anchor"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_rolesanywhere_profile", "role_arns") => Some("Profile roles"),
        ("aws_rolesanywhere_profile", "managed_policy_arns") => Some("Profile managed policies"),
        ("aws_rolesanywhere_trust_anchor", CA) => Some("Trust anchor CA"),
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == "aws_rolesanywhere_trust_anchor" && path == "source.0.source_data.0.acm_pca_arn"
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_rolesanywhere_profile", "role_arns", "aws_iam_role")
        | ("aws_rolesanywhere_profile", "managed_policy_arns", "aws_iam_policy")
        | ("aws_rolesanywhere_trust_anchor", CA, "aws_acmpca_certificate_authority") => &["arn"],
        _ => &[],
    }
}
fn shape(value: &Value, mask: &Value, path: &str) -> bool {
    if path != CA {
        return value[path].is_array();
    }
    value["source"].as_array().is_some_and(|a| a.len() == 1)
        && value["source"][0]["source_data"]
            .as_array()
            .is_some_and(|a| a.len() == 1)
        && value["source"][0]["source_data"][0]["acm_pca_arn"].is_string()
        && super::nested_references::value(value, mask, "source.0.source_type")
            .and_then(Value::as_str)
            == Some("AWS_ACM_PCA")
        && super::nested_references::value(
            value,
            mask,
            "source.0.source_data.0.x509_certificate_data",
        )
        .is_some_and(Value::is_null)
}
fn global(kind: &str) -> bool {
    matches!(kind, "aws_iam_role" | "aws_iam_policy")
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
            Some(
                shape(value, mask, &r.attribute)
                    && super::nested_references::verify(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |_, _| true,
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
            global(&right.resource_type),
        )
    })
}
