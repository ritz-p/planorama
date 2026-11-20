use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_inspector_assessment_target", "resource_group_arn") => Rule::new(
            "Inspector resource group",
            &[("aws_inspector_resource_group", &["arn"])],
            false,
            &[],
        ),
        ("aws_inspector_assessment_template", "target_arn") => Rule::new(
            "Inspector assessment target",
            &[("aws_inspector_assessment_target", &["arn"])],
            false,
            &[],
        ),
        ("aws_inspector_assessment_template", "event_subscription.topic_arn") => {
            Rule::scalar_set("Assessment notifications", &[("aws_sns_topic", &["arn"])])
        }
        (
            "aws_inspector2_delegated_admin_account" | "aws_inspector2_member_association",
            "account_id",
        ) => Rule::new(
            "Inspector account",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_inspector2_enabler", "account_ids") => Rule::new(
            "Enabled Inspector accounts",
            &[("aws_organizations_account", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_inspector_assessment_target"
            | "aws_inspector_assessment_template"
            | "aws_inspector2_delegated_admin_account"
            | "aws_inspector2_member_association"
            | "aws_inspector2_enabler"
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
    let _ = (r, path);
    e.nodes[source].resource_type != "aws_organizations_account"
        || e.text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some()
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/inspector_assessments.rs"]
mod tests;
