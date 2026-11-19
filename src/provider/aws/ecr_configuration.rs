use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ecrpublic_repository_policy", "repository_name") => Rule::new(
            "Public repository",
            &[("aws_ecrpublic_repository", &["repository_name"])],
            false,
            &[],
        ),
        (
            "aws_ecr_repository" | "aws_ecr_repository_creation_template",
            "encryption_configuration.kms_key",
        ) => Rule::new(
            "Repository encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_ecr_repository_creation_template", "custom_role_arn") => Rule::new(
            "Repository creation role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_ecr_pull_through_cache_rule", "credential_arn") => Rule::new(
            "Upstream registry secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &[],
        ),
        ("aws_ecr_pull_through_cache_rule", "custom_role_arn") => Rule::new(
            "Pull-through role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_ecr_pull_time_update_exclusion", "principal_arn") => Rule::new(
            "Pull-time exclusion principal",
            &[("aws_iam_role", &["arn"]), ("aws_iam_user", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ecrpublic_repository_policy"
            | "aws_ecr_repository"
            | "aws_ecr_repository_creation_template"
            | "aws_ecr_pull_through_cache_rule"
            | "aws_ecr_pull_time_update_exclusion"
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
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_ecrpublic_repository_policy", "repository_name") => {
            [r.target, source].into_iter().all(|index| {
                super::scope::scope(&e.nodes[index])
                    .and_then(|scope| scope.region)
                    .is_some_and(|r| r.0 == "us-east-1")
            })
        }
        (_, "encryption_configuration.kms_key") => {
            path.strip_suffix(".kms_key").is_some_and(|parent| {
                e.text(r.target, &format!("{parent}.encryption_type")) == Some("KMS")
            })
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ecr_configuration.rs"]
mod tests;
