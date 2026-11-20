use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_route53recoverycontrolconfig_control_panel"
            | "aws_route53recoverycontrolconfig_routing_control",
            "cluster_arn",
        ) => Rule::new(
            "Routing control cluster",
            &[("aws_route53recoverycontrolconfig_cluster", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (
            "aws_route53recoverycontrolconfig_routing_control"
            | "aws_route53recoverycontrolconfig_safety_rule",
            "control_panel_arn",
        ) => Rule::new(
            "Control panel",
            &[("aws_route53recoverycontrolconfig_control_panel", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_route53recoverycontrolconfig_safety_rule", "asserted_controls") => Rule::new(
            "Asserted routing controls",
            &[("aws_route53recoverycontrolconfig_routing_control", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        ("aws_route53recoverycontrolconfig_safety_rule", "gating_controls") => Rule::new(
            "Gating routing controls",
            &[("aws_route53recoverycontrolconfig_routing_control", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        ("aws_route53recoverycontrolconfig_safety_rule", "target_controls") => Rule::new(
            "Target routing controls",
            &[("aws_route53recoverycontrolconfig_routing_control", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_route53recoverycontrolconfig_control_panel"
            | "aws_route53recoverycontrolconfig_routing_control"
            | "aws_route53recoverycontrolconfig_safety_rule"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(_e: &Evidence<'_>, _r: &AttributeReference, _source: usize, _path: &str) -> bool {
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/arc_routing_control.rs"]
mod tests;
