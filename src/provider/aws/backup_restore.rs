use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const PLAN: &str = "aws_backup_restore_testing_plan";
const SELECTION: &str = "aws_backup_restore_testing_selection";
const VAULTS: &str = "recovery_point_selection.include_vaults";
const PROTECTED: &str = "protected_resource_arns";

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, PLAN | SELECTION)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == PLAN && path == "recovery_point_selection.0.include_vaults"
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (PLAN, VAULTS) => Some("Restore testing included vaults"),
        (SELECTION, "restore_testing_plan_name") => Some("Restore testing plan owner"),
        (SELECTION, "iam_role_arn") => Some("Restore testing IAM role"),
        (SELECTION, PROTECTED) => Some("Restore testing protected resources"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        (VAULTS, "aws_backup_vault" | "aws_backup_logically_air_gapped_vault") => &["arn"],
        ("restore_testing_plan_name", PLAN) => &["name"],
        ("iam_role_arn", "aws_iam_role") => &["arn"],
        (
            PROTECTED,
            "aws_instance"
            | "aws_ebs_volume"
            | "aws_efs_file_system"
            | "aws_db_instance"
            | "aws_rds_cluster"
            | "aws_docdb_cluster"
            | "aws_neptune_cluster"
            | "aws_dynamodb_table"
            | "aws_s3_bucket"
            | "aws_fsx_lustre_file_system"
            | "aws_fsx_windows_file_system"
            | "aws_fsx_ontap_file_system"
            | "aws_fsx_openzfs_file_system",
        ) => &["arn"],
        _ => &[],
    }
}
fn protected_type(value: &str, node: &TerraformEntity, sv: &Value, sm: &Value) -> bool {
    match (value, node.resource_type.as_str()) {
        ("EC2", "aws_instance")
        | ("EBS", "aws_ebs_volume")
        | ("EFS", "aws_efs_file_system")
        | ("RDS", "aws_db_instance")
        | ("DocumentDB", "aws_docdb_cluster")
        | ("Neptune", "aws_neptune_cluster")
        | ("DynamoDB", "aws_dynamodb_table")
        | ("S3", "aws_s3_bucket")
        | (
            "FSx",
            "aws_fsx_lustre_file_system"
            | "aws_fsx_windows_file_system"
            | "aws_fsx_ontap_file_system"
            | "aws_fsx_openzfs_file_system",
        ) => true,
        ("Aurora", "aws_rds_cluster") => matches!(
            super::service_evidence::known(&[(sv, sm)], 0, "engine"),
            Some("aurora" | "aurora-mysql" | "aurora-postgresql")
        ),
        _ => false,
    }
}
fn empty(value: &Value) -> bool {
    value.is_null() || value.as_array().is_some_and(Vec::is_empty)
}
fn concrete(values: &Value) -> bool {
    values.as_array().is_some_and(|a| {
        !a.is_empty()
            && a.iter().all(|v| {
                v.as_str()
                    .is_some_and(|s| s.starts_with("arn:") && !s.contains(['*', '?']))
            })
    })
}
fn qualifiers(v: &Value, m: &Value, path: &str) -> bool {
    if path == VAULTS {
        if !super::nested_references::singleton(v, m, "recovery_point_selection") {
            return false;
        }
        let Some(included) =
            super::nested_references::value(v, m, "recovery_point_selection.0.include_vaults")
                .filter(|v| concrete(v))
        else {
            return false;
        };
        let Some(excluded) =
            super::nested_references::value(v, m, "recovery_point_selection.0.exclude_vaults")
        else {
            return false;
        };
        return empty(excluded)
            || excluded.as_array().is_some_and(|a| {
                a.iter().all(|e| {
                    e.as_str()
                        .is_some_and(|s| s.starts_with("arn:") && !s.contains(['*', '?']))
                        && !a.iter().any(|e| included.as_array().unwrap().contains(e))
                })
            });
    }
    if path == PROTECTED {
        return super::nested_references::value(v, m, PROTECTED).is_some_and(concrete)
            && super::nested_references::value(v, m, "protected_resource_conditions")
                .is_some_and(empty);
    }
    true
}
fn compatible(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let l = super::scope::scope(left);
    let r = super::scope::scope(right);
    super::eventbridge::bindings(
        left.provider_configuration.as_ref(),
        right.provider_configuration.as_ref(),
        l.as_ref().and_then(|s| s.region.as_ref()),
        r.as_ref().and_then(|s| s.region.as_ref()),
        right.resource_type == "aws_iam_role",
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let (v, m) = evidence[r.target];
            Some(
                qualifiers(v, m, &r.attribute)
                    && super::nested_references::verify(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |s, _| {
                            compatible(&nodes[r.target], &nodes[s])
                                && (r.attribute != PROTECTED
                                    || super::service_evidence::known(
                                        evidence,
                                        r.target,
                                        "protected_resource_type",
                                    )
                                    .is_some_and(|kind| {
                                        let (sv, sm) = evidence[s];
                                        protected_type(kind, &nodes[s], sv, sm)
                                    }))
                        },
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
            let l = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.resource_type == "aws_iam_role",
            )
        })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/backup_restore.rs"]
mod tests;
