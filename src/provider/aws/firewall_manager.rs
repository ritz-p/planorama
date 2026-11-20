use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_fms_admin_account", "account_id") => Rule::new(
            "Firewall Manager administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_fms_policy", "resource_set_ids") => Rule::new(
            "Firewall Manager resource sets",
            &[("aws_fms_resource_set", &["id"])],
            true,
            &[],
        ),
        ("aws_fms_policy", "include_map.account") => Rule::new(
            "Included accounts",
            &[("aws_organizations_account", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        ("aws_fms_policy", "include_map.orgunit") => Rule::new(
            "Included organizational units",
            &[("aws_organizations_organizational_unit", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        ("aws_fms_policy", "exclude_map.account") => Rule::new(
            "Excluded accounts",
            &[("aws_organizations_account", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        ("aws_fms_policy", "exclude_map.orgunit") => Rule::new(
            "Excluded organizational units",
            &[("aws_organizations_organizational_unit", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_fms_admin_account" | "aws_fms_policy")
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
    if e.nodes[source].resource_type == "aws_organizations_account"
        && e.text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_none()
    {
        return false;
    }
    if e.nodes[source].resource_type == "aws_organizations_organizational_unit"
        && !e.text(source, "id").is_some_and(|v| v.starts_with("ou-"))
    {
        return false;
    }
    if r.attribute.starts_with("include_map.") {
        return e.absent(r.target, "exclude_map");
    }
    if r.attribute.starts_with("exclude_map.") {
        return e.absent(r.target, "include_map");
    }
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/firewall_manager.rs"]
mod tests;
