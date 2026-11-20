use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_guardduty_malware_protection_plan", "role") => Rule::new(
            "Malware protection role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_guardduty_malware_protection_plan", "protected_resource.s3_bucket.bucket_name") => {
            Rule::new(
                "Protected bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &["protected_resource", "protected_resource.s3_bucket"],
            )
        }
        ("aws_guardduty_publishing_destination", "detector_id") => Rule::new(
            "Publishing detector",
            &[("aws_guardduty_detector", &["id"])],
            false,
            &[],
        ),
        ("aws_guardduty_publishing_destination", "destination_arn") => Rule::new(
            "Finding export bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination()
        .decoded(super::finding_exports::bucket_arn),
        ("aws_guardduty_publishing_destination", "kms_key_arn") => Rule::new(
            "Finding export key",
            &[("aws_kms_key", &["arn"])],
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
        "aws_guardduty_malware_protection_plan" | "aws_guardduty_publishing_destination"
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
    match path {
        "destination_arn" => {
            e.text(r.target, "destination_type") == Some("S3")
                && super::finding_exports::region_matches(
                    e,
                    r.target,
                    source,
                    "destination_arn",
                    "kms_key_arn",
                )
        }
        "kms_key_arn" => super::finding_exports::region_matches(
            e,
            r.target,
            source,
            "destination_arn",
            "kms_key_arn",
        ),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/guardduty_destinations.rs"]
mod tests;
