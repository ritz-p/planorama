use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_shield_application_layer_automatic_response", "resource_arn") => Rule::new(
            "Automatic response target",
            &[
                ("aws_lb", &["arn"]),
                ("aws_cloudfront_distribution", &["arn"]),
            ],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_protection", "resource_arn") => Rule::new(
            "Protected endpoint",
            &[
                ("aws_lb", &["arn"]),
                ("aws_elb", &["arn"]),
                ("aws_cloudfront_distribution", &["arn"]),
                ("aws_eip", &["arn"]),
                ("aws_route53_zone", &["arn"]),
                ("aws_globalaccelerator_accelerator", &["arn"]),
            ],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_protection_group", "members") => Rule::new(
            "Protected group members",
            &[
                ("aws_lb", &["arn"]),
                ("aws_elb", &["arn"]),
                ("aws_cloudfront_distribution", &["arn"]),
                ("aws_eip", &["arn"]),
                ("aws_route53_zone", &["arn"]),
                ("aws_globalaccelerator_accelerator", &["arn"]),
            ],
            true,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_drt_access_log_bucket_association", "log_bucket") => Rule::new(
            "DRT log bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_drt_access_log_bucket_association", "role_arn_association_id") => Rule::new(
            "DRT authorization",
            &[("aws_shield_drt_access_role_arn_association", &["id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_drt_access_role_arn_association", "role_arn") => {
            Rule::new("DRT role", &[("aws_iam_role", &["arn"])], false, &[])
                .in_control_plane_account()
        }
        ("aws_shield_protection_health_check_association", "health_check_arn") => Rule::new(
            "Protection health check",
            &[("aws_route53_health_check", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_shield_protection_health_check_association", "shield_protection_id") => Rule::new(
            "Shield protection",
            &[("aws_shield_protection", &["id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_shield_application_layer_automatic_response"
            | "aws_shield_protection"
            | "aws_shield_protection_group"
            | "aws_shield_drt_access_log_bucket_association"
            | "aws_shield_drt_access_role_arn_association"
            | "aws_shield_protection_health_check_association"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let kind = &e.nodes[r.target].resource_type;
    let target = &e.nodes[source].resource_type;
    if r.attribute == "members" && e.text(r.target, "pattern") != Some("ARBITRARY") {
        return false;
    }
    if target == "aws_lb" {
        return match e.text(source, "load_balancer_type") {
            Some("application") => true,
            Some("network") => kind != "aws_shield_application_layer_automatic_response",
            _ => false,
        };
    }
    let _ = path;
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/shield.rs"]
mod tests;
