use crate::model::{AttributeReference, RegionScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/route53_health.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    kind == "aws_route53_health_check"
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if !resource(kind) {
        return None;
    }
    match path {
        "child_healthchecks" => Some("Calculated child health checks"),
        "cloudwatch_alarm_name" => Some("Health check metric alarm"),
        "routing_control_arn" => Some("Health check routing control"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("child_healthchecks", "aws_route53_health_check") => &["id"],
        ("cloudwatch_alarm_name", "aws_cloudwatch_metric_alarm") => &["alarm_name"],
        ("routing_control_arn", "aws_route53recoverycontrolconfig_routing_control") => &["arn"],
        _ => &[],
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
            let mode = super::service_evidence::known(e, r.target, "type");
            let allowed = match r.attribute.as_str() {
                "child_healthchecks" => mode == Some("CALCULATED"),
                "cloudwatch_alarm_name" => mode == Some("CLOUDWATCH_METRIC"),
                "routing_control_arn" => mode == Some("RECOVERY_CONTROL"),
                _ => false,
            };
            let region = super::service_evidence::known(e, r.target, "cloudwatch_alarm_region")
                .map(|s| RegionScope(s.into()));
            let valid = allowed
                && (r.attribute != "cloudwatch_alarm_name" || region.is_some())
                && super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                    let (v, m) = e[r.target];
                    super::nested_references::value(v, m, path).is_some_and(|v| {
                        if r.attribute == "child_healthchecks" {
                            v.is_array()
                        } else {
                            v.is_string()
                        }
                    })
                })
                .is_some();
            Some((valid, region))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, region)) = result {
            r.service_reference_verified = valid;
            r.service_reference_region = region;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let target = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let source = &raw.nodes[s];
        super::iam_policies::same_account(
            target.provider_configuration.as_ref(),
            source.provider_configuration.as_ref(),
        ) && (r.attribute != "cloudwatch_alarm_name"
            || r.service_reference_region.as_ref().is_some_and(|region| {
                source.entity.scope.as_ref().and_then(|s| s.region.as_ref()) == Some(region)
            }))
    })
}
