use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_macie2_classification_export_configuration", "s3_destination.bucket_name") => {
            Rule::new(
                "Classification export bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
            .with_explicit_destination()
        }
        ("aws_macie2_classification_export_configuration", "s3_destination.kms_key_arn") => {
            Rule::new(
                "Classification export key",
                &[("aws_kms_key", &["arn"])],
                false,
                &[],
            )
            .with_explicit_destination()
        }
        ("aws_macie2_classification_job", "custom_data_identifier_ids") => Rule::new(
            "Custom data identifiers",
            &[("aws_macie2_custom_data_identifier", &["id"])],
            true,
            &[],
        ),
        ("aws_macie2_classification_job", "s3_job_definition.bucket_definitions.buckets") => {
            Rule::new(
                "Classification buckets",
                &[("aws_s3_bucket", &["bucket", "id"])],
                true,
                &["s3_job_definition.bucket_definitions"],
            )
            .with_explicit_destination()
        }
        ("aws_macie2_classification_job", "s3_job_definition.bucket_definitions.account_id") => {
            Rule::new(
                "Bucket definition account",
                &[("aws_organizations_account", &["id"])],
                false,
                &["s3_job_definition.bucket_definitions"],
            )
            .with_explicit_destination()
        }
        ("aws_macie2_invitation_accepter", "administrator_account_id") => Rule::new(
            "Macie administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_macie2_member", "account_id") => Rule::new(
            "Macie member",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_macie2_organization_admin_account", "admin_account_id") => Rule::new(
            "Organization administrator",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_macie2_classification_export_configuration"
            | "aws_macie2_classification_job"
            | "aws_macie2_invitation_accepter"
            | "aws_macie2_member"
            | "aws_macie2_organization_admin_account"
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
    if matches!(
        path,
        "s3_destination.0.bucket_name" | "s3_destination.0.kms_key_arn"
    ) {
        return super::finding_exports::region_matches(
            e,
            r.target,
            source,
            "s3_destination.0.bucket_name",
            "s3_destination.0.kms_key_arn",
        );
    }
    if r.attribute == "s3_job_definition.bucket_definitions.buckets" {
        let Some((prefix, _)) = path.rsplit_once('.') else {
            return false;
        };
        return e
            .text(r.target, &format!("{prefix}.account_id"))
            .zip(
                e.nodes[source]
                    .provider_configuration
                    .as_ref()
                    .and_then(|p| p.account.as_ref()),
            )
            .is_some_and(|(want, actual)| want == actual.as_str())
            && super::scope(&e.nodes[r.target])
                .and_then(|s| s.region)
                .zip(super::scope(&e.nodes[source]).and_then(|s| s.region))
                .is_some_and(|(a, b)| a == b);
    }
    e.nodes[source].resource_type != "aws_organizations_account"
        || e.text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some()
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/macie_classification.rs"]
mod tests;
