use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_securityhub_configuration_policy_association", "policy_id") => Rule::new(
            "Security Hub configuration policy",
            &[("aws_securityhub_configuration_policy", &["id"])],
            false,
            &[],
        ),
        ("aws_securityhub_configuration_policy_association", "target_id") => Rule::new(
            "Configuration policy target",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organizational_unit", &["id"]),
                ("aws_organizations_organization", &["roots[].id"]),
            ],
            false,
            &[],
        ),
        ("aws_securityhub_invite_accepter", "master_id") => Rule::new(
            "Inviting administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_securityhub_member", "account_id") => Rule::new(
            "Security Hub member",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_securityhub_organization_admin_account", "admin_account_id") => Rule::new(
            "Security Hub administrator",
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
        "aws_securityhub_configuration_policy_association"
            | "aws_securityhub_invite_accepter"
            | "aws_securityhub_member"
            | "aws_securityhub_organization_admin_account"
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
    match (r.attribute.as_str(), e.nodes[source].resource_type.as_str()) {
        ("policy_id", _) => e
            .text(r.target, "policy_id")
            .is_some_and(|v| v != "SELF_MANAGED_SECURITY_HUB"),
        (_, "aws_organizations_account") => e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some(),
        ("target_id", "aws_organizations_organizational_unit") => {
            e.text(source, "id").is_some_and(|v| v.starts_with("ou-"))
        }
        ("target_id", "aws_organizations_organization") => e
            .text(r.target, "target_id")
            .is_some_and(|v| v.starts_with("r-")),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/securityhub_organization.rs"]
mod tests;
