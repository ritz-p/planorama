use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_guardduty_detector_feature"
            | "aws_guardduty_filter"
            | "aws_guardduty_invite_accepter"
            | "aws_guardduty_ipset"
            | "aws_guardduty_member"
            | "aws_guardduty_member_detector_feature"
            | "aws_guardduty_organization_configuration"
            | "aws_guardduty_organization_configuration_feature"
            | "aws_guardduty_threatintelset",
            "detector_id",
        ) => Rule::new(
            "GuardDuty detector",
            &[("aws_guardduty_detector", &["id"])],
            false,
            &[],
        ),
        ("aws_guardduty_member" | "aws_guardduty_member_detector_feature", "account_id") => {
            Rule::new(
                "GuardDuty member account",
                &[("aws_organizations_account", &["id"])],
                false,
                &[],
            )
        }
        ("aws_guardduty_organization_admin_account", "admin_account_id") => Rule::new(
            "GuardDuty administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_guardduty_invite_accepter", "master_account_id") => Rule::new(
            "Inviting administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_guardduty_detector_feature"
            | "aws_guardduty_filter"
            | "aws_guardduty_invite_accepter"
            | "aws_guardduty_ipset"
            | "aws_guardduty_member"
            | "aws_guardduty_member_detector_feature"
            | "aws_guardduty_organization_configuration"
            | "aws_guardduty_organization_configuration_feature"
            | "aws_guardduty_threatintelset"
            | "aws_guardduty_organization_admin_account"
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
    let _ = path;
    if e.nodes[source].resource_type == "aws_organizations_account" {
        return e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some();
    }
    if e.nodes[r.target].resource_type == "aws_guardduty_member_detector_feature" {
        return e
            .text(r.target, "account_id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some();
    }
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/guardduty_members.rs"]
mod tests;
