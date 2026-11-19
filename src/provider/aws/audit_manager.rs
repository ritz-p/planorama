use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_auditmanager_account_registration", "kms_key") => Rule::new(
            "Registration encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_account_registration", "delegated_admin_account") => Rule::new(
            "Delegated administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_assessment", "framework_id") => Rule::new(
            "Assessment framework",
            &[("aws_auditmanager_framework", &["id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_assessment", "roles.role_arn") => Rule::new(
            "Assessment role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["roles"],
        ),
        ("aws_auditmanager_assessment", "scope.aws_accounts.id") => Rule::new(
            "Assessment account",
            &[("aws_organizations_account", &["id"])],
            false,
            &["scope", "scope.aws_accounts"],
        ),
        ("aws_auditmanager_assessment", "assessment_reports_destination.destination") => Rule::new(
            "Assessment reports bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["assessment_reports_destination"],
        )
        .decoded(super::athena_queries::decode),
        (
            "aws_auditmanager_assessment_delegation" | "aws_auditmanager_assessment_report",
            "assessment_id",
        ) => Rule::new(
            "Assessment",
            &[("aws_auditmanager_assessment", &["id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_assessment_delegation", "role_arn") => Rule::new(
            "Delegated assessment role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_auditmanager_framework", "control_sets.controls.id") => Rule::new(
            "Framework control",
            &[("aws_auditmanager_control", &["id"])],
            false,
            &["control_sets", "control_sets.controls"],
        ),
        ("aws_auditmanager_framework_share", "framework_id") => Rule::new(
            "Shared framework",
            &[("aws_auditmanager_framework", &["id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_framework_share", "destination_account") => Rule::new(
            "Framework recipient",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_auditmanager_organization_admin_account_registration", "admin_account_id") => {
            Rule::new(
                "Audit Manager administrator",
                &[("aws_organizations_account", &["id"])],
                false,
                &[],
            )
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_auditmanager_account_registration"
            | "aws_auditmanager_assessment"
            | "aws_auditmanager_assessment_delegation"
            | "aws_auditmanager_assessment_report"
            | "aws_auditmanager_framework"
            | "aws_auditmanager_framework_share"
            | "aws_auditmanager_organization_admin_account_registration"
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
    match r.attribute.as_str() {
        "assessment_reports_destination.destination" => {
            e.text(
                r.target,
                "assessment_reports_destination.0.destination_type",
            ) == Some("S3")
        }
        "destination_account" => e
            .text(r.target, "destination_region")
            .and_then(super::scope_selections::region)
            .is_some(),
        _ if e.nodes[source].resource_type == "aws_organizations_account" => e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some(),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/audit_manager.rs"]
mod tests;
