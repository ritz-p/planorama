use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_signer_signing_job" | "aws_signer_signing_profile_permission", "profile_name") => {
            Rule::new(
                "Signing profile",
                &[("aws_signer_signing_profile", &["name"])],
                false,
                &[],
            )
        }
        ("aws_signer_signing_job", "source.s3.bucket") => Rule::new(
            "Signing source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_signer_signing_job", "destination.s3.bucket") => Rule::new(
            "Signing destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_signer_signing_profile", "signing_material.certificate_arn") => Rule::new(
            "Signing certificate",
            &[("aws_acm_certificate", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_signer_signing_job"
            | "aws_signer_signing_profile_permission"
            | "aws_signer_signing_profile"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    e.nodes[r.target].resource_type != "aws_signer_signing_profile_permission"
        || e.value(r.target, "profile_version").is_some_and(|version| {
            version.is_null()
                || version
                    .as_str()
                    .is_some_and(|version| e.text(source, "version") == Some(version))
        })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/signer_configuration.rs"]
mod tests;
