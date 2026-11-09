use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/rum.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_rum_app_monitor" | "aws_rum_metrics_destination")
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_rum_app_monitor", "app_monitor_configuration.guest_role_arn") => {
            Some("RUM guest role")
        }
        ("aws_rum_app_monitor", "app_monitor_configuration.identity_pool_id") => {
            Some("RUM identity pool")
        }
        ("aws_rum_metrics_destination", "app_monitor_name") => Some("RUM app monitor"),
        ("aws_rum_metrics_destination", "iam_role_arn") => Some("Evidently destination role"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("app_monitor_configuration.guest_role_arn" | "iam_role_arn", "aws_iam_role") => &["arn"],
        ("app_monitor_configuration.identity_pool_id", "aws_cognito_identity_pool") => &["id"],
        ("app_monitor_name", "aws_rum_app_monitor") => &["name"],
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
            let mode = super::service_evidence::known(e, r.target, "destination");
            let allowed = if nodes[r.target].resource_type == "aws_rum_app_monitor" {
                v["app_monitor_configuration"]
                    .as_array()
                    .is_some_and(|a| a.len() == 1)
            } else {
                matches!(mode, Some("CloudWatch" | "Evidently"))
                    && (r.attribute != "iam_role_arn" || mode == Some("Evidently"))
            };
            Some(
                allowed
                    && super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                        super::nested_references::value(v, m, path).is_some_and(Value::is_string)
                    })
                    .is_some(),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
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
