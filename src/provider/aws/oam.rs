use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/oam.rs"]
mod tests;
pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_oam_link" | "aws_oam_sink" | "aws_oam_sink_policy"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_oam_link", "sink_identifier") => Some("OAM monitoring sink"),
        ("aws_oam_sink_policy", "sink_identifier") => Some("OAM policy sink"),
        _ => None,
    }
}
fn fields(_: &str, _: &str, source: &str) -> &'static [&'static str] {
    if source == "aws_oam_sink" {
        &["arn", "id"]
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
                super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                    super::nested_references::value(e[r.target].0, e[r.target].1, path)
                        .is_some_and(Value::is_string)
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
    let a = &raw.nodes[r.target];
    if label(&a.resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let b = &raw.nodes[s];
        let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        ar.is_some()
            && ar == br
            && if a.resource_type == "aws_oam_link" {
                a.provider_configuration
                    .as_ref()
                    .zip(b.provider_configuration.as_ref())
                    .is_some_and(|(a, b)| {
                        !a.account_conflict
                            && !b.account_conflict
                            && a.account.is_some()
                            && b.account.is_some()
                    })
            } else {
                super::iam_policies::same_account(
                    a.provider_configuration.as_ref(),
                    b.provider_configuration.as_ref(),
                )
            }
    })
}
