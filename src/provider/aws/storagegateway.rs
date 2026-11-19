use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_storagegateway_gateway", "cloudwatch_log_group_arn") => Rule::new(
            "Gateway log group",
            &[("aws_cloudwatch_log_group", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_storagegateway_cache"
            | "aws_storagegateway_upload_buffer"
            | "aws_storagegateway_working_storage"
            | "aws_storagegateway_cached_iscsi_volume"
            | "aws_storagegateway_stored_iscsi_volume"
            | "aws_storagegateway_nfs_file_share"
            | "aws_storagegateway_smb_file_share"
            | "aws_storagegateway_file_system_association",
            "gateway_arn",
        ) => Rule::new(
            "Storage Gateway",
            &[("aws_storagegateway_gateway", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_storagegateway_cached_iscsi_volume" | "aws_storagegateway_stored_iscsi_volume",
            "snapshot_id",
        ) => Rule::new(
            "Restore snapshot",
            &[("aws_ebs_snapshot", &["id"])],
            false,
            &[],
        ),
        (
            "aws_storagegateway_cached_iscsi_volume" | "aws_storagegateway_stored_iscsi_volume",
            "kms_key",
        ) => Rule::new(
            "Volume encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_storagegateway_cached_iscsi_volume", "source_volume_arn") => Rule::new(
            "Recovery source volume",
            &[
                ("aws_storagegateway_cached_iscsi_volume", &["arn"]),
                ("aws_storagegateway_stored_iscsi_volume", &["arn"]),
            ],
            false,
            &[],
        ),
        (
            "aws_storagegateway_nfs_file_share" | "aws_storagegateway_smb_file_share",
            "location_arn",
        ) => Rule::new(
            "Backing storage",
            &[
                ("aws_s3_bucket", &["arn"]),
                ("aws_s3_access_point", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_storagegateway_nfs_file_share" | "aws_storagegateway_smb_file_share", "role_arn") => {
            Rule::new("File share role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        (
            "aws_storagegateway_nfs_file_share" | "aws_storagegateway_smb_file_share",
            "kms_key_arn",
        ) => Rule::new(
            "File share encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_storagegateway_nfs_file_share" | "aws_storagegateway_smb_file_share",
            "audit_destination_arn",
        ) => Rule::new(
            "File share audit logs",
            &[("aws_cloudwatch_log_group", &["arn"])],
            false,
            &[],
        ),
        ("aws_storagegateway_file_system_association", "location_arn") => Rule::new(
            "Associated Windows filesystem",
            &[("aws_fsx_windows_file_system", &["arn"])],
            false,
            &[],
        ),
        ("aws_storagegateway_file_system_association", "audit_destination_arn") => Rule::new(
            "Filesystem audit destination",
            &[
                ("aws_cloudwatch_log_group", &["arn"]),
                ("aws_s3_bucket", &["arn"]),
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
        "aws_storagegateway_gateway"
            | "aws_storagegateway_cache"
            | "aws_storagegateway_upload_buffer"
            | "aws_storagegateway_working_storage"
            | "aws_storagegateway_cached_iscsi_volume"
            | "aws_storagegateway_stored_iscsi_volume"
            | "aws_storagegateway_nfs_file_share"
            | "aws_storagegateway_smb_file_share"
            | "aws_storagegateway_file_system_association"
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
    let absent = |path| e.value(r.target, path).is_some_and(Value::is_null);
    match r.attribute.as_str() {
        "kms_key" | "kms_key_arn" => {
            e.value(r.target, "kms_encrypted").and_then(Value::as_bool) == Some(true)
        }
        "snapshot_id"
            if e.nodes[r.target].resource_type == "aws_storagegateway_cached_iscsi_volume" =>
        {
            absent("source_volume_arn")
        }
        "source_volume_arn" => absent("snapshot_id"),
        "location_arn"
            if e.nodes[r.target].resource_type != "aws_storagegateway_file_system_association" =>
        {
            absent("bucket_region")
                || e.text(r.target, "bucket_region").is_some_and(|region| {
                    super::scope::scope(&e.nodes[source])
                        .and_then(|scope| scope.region)
                        .is_some_and(|scope| scope.0 == region)
                })
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/storagegateway.rs"]
mod tests;
