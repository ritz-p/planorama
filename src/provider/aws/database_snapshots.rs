use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/database_snapshots.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_db_snapshot", "db_instance_identifier") => Some("Snapshot DB instance"),
        (
            "aws_db_cluster_snapshot"
            | "aws_docdb_cluster_snapshot"
            | "aws_neptune_cluster_snapshot",
            "db_cluster_identifier",
        ) => Some("Snapshot cluster"),
        ("aws_db_snapshot_copy", "source_db_snapshot_identifier")
        | ("aws_rds_cluster_snapshot_copy", "source_db_cluster_snapshot_identifier") => {
            Some("Source snapshot")
        }
        ("aws_rds_export_task", "source_arn") => Some("Export source"),
        ("aws_rds_export_task", "s3_bucket_name") => Some("Export bucket"),
        ("aws_rds_export_task", "iam_role_arn") => Some("Export role"),
        ("aws_rds_export_task", "kms_key_id") => Some("Export KMS key"),
        _ => None,
    }
}

fn output(kind: &str, field: &str, source: &str, attribute: &str) -> bool {
    match (kind, field, source) {
        ("aws_db_snapshot", "db_instance_identifier", "aws_db_instance") => {
            attribute == "identifier"
        }
        ("aws_db_cluster_snapshot", "db_cluster_identifier", "aws_rds_cluster")
        | ("aws_docdb_cluster_snapshot", "db_cluster_identifier", "aws_docdb_cluster")
        | ("aws_neptune_cluster_snapshot", "db_cluster_identifier", "aws_neptune_cluster") => {
            matches!(attribute, "id" | "cluster_identifier")
        }
        ("aws_db_snapshot_copy", "source_db_snapshot_identifier", "aws_db_snapshot") => matches!(
            attribute,
            "db_snapshot_arn" | "db_snapshot_identifier" | "id"
        ),
        ("aws_db_snapshot_copy", "source_db_snapshot_identifier", "aws_db_snapshot_copy") => {
            matches!(
                attribute,
                "db_snapshot_arn" | "target_db_snapshot_identifier" | "id"
            )
        }
        (
            "aws_rds_cluster_snapshot_copy",
            "source_db_cluster_snapshot_identifier",
            "aws_db_cluster_snapshot",
        ) => matches!(
            attribute,
            "db_cluster_snapshot_arn" | "db_cluster_snapshot_identifier" | "id"
        ),
        (
            "aws_rds_cluster_snapshot_copy",
            "source_db_cluster_snapshot_identifier",
            "aws_rds_cluster_snapshot_copy",
        ) => matches!(
            attribute,
            "db_cluster_snapshot_arn" | "target_db_cluster_snapshot_identifier" | "id"
        ),
        ("aws_rds_export_task", "source_arn", "aws_db_snapshot" | "aws_db_snapshot_copy") => {
            attribute == "db_snapshot_arn"
        }
        (
            "aws_rds_export_task",
            "source_arn",
            "aws_db_cluster_snapshot" | "aws_rds_cluster_snapshot_copy",
        ) => attribute == "db_cluster_snapshot_arn",
        ("aws_rds_export_task", "s3_bucket_name", "aws_s3_bucket") => {
            matches!(attribute, "id" | "bucket")
        }
        ("aws_rds_export_task", "iam_role_arn", "aws_iam_role") => attribute == "arn",
        ("aws_rds_export_task", "kms_key_id", "aws_kms_key") => {
            matches!(attribute, "arn" | "key_id")
        }
        _ => false,
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        let [source] = r.sources.as_slice() else {
            continue;
        };
        let endpoint = &nodes[*source];
        let Some(field) = r.source_attributes.iter().next() else {
            continue;
        };
        if !current(node)
            || !current(endpoint)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.source_attributes.len() != 1
            || !output(
                &node.resource_type,
                &r.attribute,
                &endpoint.resource_type,
                field,
            )
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        let (source_value, source_mask) = evidence[*source];
        let Some(actual) = super::service_references::scalar(value, mask, &[&r.attribute]) else {
            continue;
        };
        if super::service_references::scalar(source_value, source_mask, &[field]) != Some(actual) {
            continue;
        }
        let left = super::scope::scope(node);
        let right = super::scope::scope(endpoint);
        let left_region = left.as_ref().and_then(|s| s.region.as_ref());
        let right_region = right.as_ref().and_then(|s| s.region.as_ref());
        let copy = matches!(
            node.resource_type.as_str(),
            "aws_db_snapshot_copy" | "aws_rds_cluster_snapshot_copy"
        );
        let cross_region = copy && left_region != right_region;
        if left_region.is_none()
            || (endpoint.resource_type != "aws_iam_role" && !copy && left_region != right_region)
        {
            continue;
        }
        if cross_region || (copy && field.ends_with("_arn")) {
            let parts: Vec<_> = actual.splitn(6, ':').collect();
            if !field.ends_with("_arn")
                || parts.len() != 6
                || parts[0] != "arn"
                || !matches!(parts[1], "aws" | "aws-cn" | "aws-us-gov")
                || parts[2] != "rds"
                || right_region.is_none_or(|region| region.0 != parts[3])
                || !parts[5].starts_with(if field == "db_snapshot_arn" {
                    "snapshot:"
                } else {
                    "cluster-snapshot:"
                })
            {
                continue;
            }
            if cross_region {
                let Some(account) = crate::model::AwsAccountScope::new(parts[4]) else {
                    continue;
                };
                r.service_reference_owner_account = Some(account);
            }
        }
        r.service_reference_verified = true;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role" || r.service_reference_owner_account.is_some(),
        ) && r
            .service_reference_owner_account
            .as_ref()
            .is_none_or(|account| {
                endpoint
                    .provider_configuration
                    .as_ref()
                    .and_then(|p| p.account.as_ref())
                    == Some(account)
            })
    })
}
