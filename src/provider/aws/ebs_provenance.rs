use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ebs_default_kms_key", "key_arn") => Rule::new(
            "Regional default key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_ebs_fast_snapshot_restore", "snapshot_id") => Rule::new(
            "Restore snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_ebs_snapshot", "volume_id") => Rule::new(
            "Source volume",
            &[
                ("aws_ebs_volume", &["id"]),
                ("aws_ebs_volume_copy", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_ebs_snapshot_copy", "source_snapshot_id") => Rule::new(
            "Source snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ebs_snapshot_copy" | "aws_ebs_snapshot_import" | "aws_ebs_volume", "kms_key_id") => {
            Rule::new("Encryption key", &[("aws_kms_key", &["arn"])], false, &[])
        }
        ("aws_ebs_snapshot_import", "disk_container.user_bucket.s3_bucket") => Rule::new(
            "Import bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_ebs_snapshot_import", "role_name") => {
            Rule::new("Import role", &[("aws_iam_role", &["name"])], false, &[])
        }
        ("aws_ebs_volume", "snapshot_id") => Rule::new(
            "Source snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_ebs_volume_copy", "source_volume_id") => Rule::new(
            "Source volume",
            &[
                ("aws_ebs_volume", &["id"]),
                ("aws_ebs_volume_copy", &["id"]),
            ],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ebs_default_kms_key"
            | "aws_ebs_fast_snapshot_restore"
            | "aws_ebs_snapshot"
            | "aws_ebs_snapshot_copy"
            | "aws_ebs_snapshot_import"
            | "aws_ebs_volume"
            | "aws_ebs_volume_copy"
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
        && (raw.nodes[r.target].resource_type != "aws_ebs_snapshot_copy"
            || r.attribute != "source_snapshot_id"
            || super::ec2_images::copy_account(
                raw,
                r,
                "aws_snapshot_create_volume_permission",
                "snapshot_id",
            ))
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    match r.attribute.as_str() {
        "source_snapshot_id" => e.source_region(r.target, "source_region", source),
        "kms_key_id" => e.value(r.target, "encrypted").and_then(Value::as_bool) == Some(true),
        "disk_container.user_bucket.s3_bucket" => e.absent(r.target, "disk_container.0.url"),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ebs_provenance.rs"]
mod tests;
