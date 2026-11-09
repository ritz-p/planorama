use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/xray_encryption.rs"]
mod tests;
pub(super) fn resource(kind: &str) -> bool {
    kind == "aws_xray_encryption_config"
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    (resource(kind) && path == "key_id").then_some("X-Ray customer KMS key")
}
fn fields(_: &str, _: &str, source: &str) -> &'static [&'static str] {
    if super::kms::key_outputs(source).contains(&"arn") {
        &["arn"]
    } else {
        &[]
    }
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
            Some(
                super::service_evidence::known(e, r.target, "type") == Some("KMS")
                    && super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                        let (v, m) = e[r.target];
                        super::nested_references::value(v, m, path).is_some_and(Value::is_string)
                    })
                    .is_some(),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let a = &raw.nodes[r.target];
        let b = &raw.nodes[s];
        let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        super::iam_policies::same_account(
            a.provider_configuration.as_ref(),
            b.provider_configuration.as_ref(),
        ) && ar.is_some()
            && ar == br
    })
}
