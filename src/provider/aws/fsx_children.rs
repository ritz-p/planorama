use super::super::{service_evidence, streaming};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/fsx_children.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_fsx_ontap_storage_virtual_machine", "file_system_id") => {
            Some("ONTAP filesystem membership")
        }
        ("aws_fsx_ontap_volume", "storage_virtual_machine_id") => Some("ONTAP SVM membership"),
        ("aws_fsx_openzfs_volume", "parent_volume_id") => Some("OpenZFS parent membership"),
        ("aws_fsx_openzfs_volume", "origin_snapshot.snapshot_arn") => {
            Some("Origin snapshot reference")
        }
        ("aws_fsx_openzfs_snapshot", "volume_id") => Some("Snapshot volume"),
        ("aws_fsx_backup", "file_system_id" | "volume_id") => Some("Backup source"),
        ("aws_fsx_data_repository_association", "file_system_id") => {
            Some("Repository association filesystem")
        }
        ("aws_fsx_s3_access_point_attachment", "openzfs_configuration.volume_id") => {
            Some("Attached OpenZFS volume")
        }
        ("aws_fsx_s3_access_point_attachment", "s3_access_point.vpc_configuration.vpc_id") => {
            Some("Access restriction VPC")
        }
        _ => None,
    }
}

fn endpoint(kind: &str, path: &str, source: &str) -> Option<&'static str> {
    match (kind, path, source) {
        (
            "aws_fsx_ontap_storage_virtual_machine",
            "file_system_id",
            "aws_fsx_ontap_file_system",
        )
        | (
            "aws_fsx_ontap_volume",
            "storage_virtual_machine_id",
            "aws_fsx_ontap_storage_virtual_machine",
        )
        | (
            "aws_fsx_backup",
            "file_system_id",
            "aws_fsx_lustre_file_system" | "aws_fsx_windows_file_system",
        )
        | ("aws_fsx_backup", "volume_id", "aws_fsx_ontap_volume")
        | ("aws_fsx_data_repository_association", "file_system_id", "aws_fsx_lustre_file_system")
        | (
            "aws_fsx_s3_access_point_attachment",
            "s3_access_point.vpc_configuration.vpc_id",
            "aws_vpc",
        ) => Some("id"),
        ("aws_fsx_openzfs_volume", "origin_snapshot.snapshot_arn", "aws_fsx_openzfs_snapshot") => {
            Some("arn")
        }
        ("aws_fsx_openzfs_volume", "parent_volume_id", "aws_fsx_openzfs_volume")
        | ("aws_fsx_openzfs_snapshot", "volume_id", "aws_fsx_openzfs_volume")
        | (
            "aws_fsx_s3_access_point_attachment",
            "openzfs_configuration.volume_id",
            "aws_fsx_openzfs_volume",
        ) => Some("id"),
        ("aws_fsx_openzfs_volume", "parent_volume_id", "aws_fsx_openzfs_file_system")
        | ("aws_fsx_openzfs_snapshot", "volume_id", "aws_fsx_openzfs_file_system")
        | (
            "aws_fsx_s3_access_point_attachment",
            "openzfs_configuration.volume_id",
            "aws_fsx_openzfs_file_system",
        ) => Some("root_volume_id"),
        _ => None,
    }
}

fn scalar<'a>(evidence: &[(&'a Value, &'a Value)], target: usize, path: &str) -> Option<&'a str> {
    let (value, unknown) = evidence[target];
    streaming::values(value, unknown, path, false)?
        .into_iter()
        .next()
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs.iter_mut() {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !service_evidence::eligible(nodes, r) || counts[&(r.target, r.attribute.clone())] != 1 {
            continue;
        }
        let [source] = r.sources.as_slice() else {
            continue;
        };
        if *source == r.target {
            continue;
        }
        let Some(output) = endpoint(kind, &r.attribute, &nodes[*source].resource_type) else {
            continue;
        };
        if r.source_attributes.len() != 1 || !r.source_attributes.contains(output) {
            continue;
        }
        let Some(actual) = scalar(evidence, r.target, &r.attribute) else {
            continue;
        };
        if scalar(evidence, *source, output) != Some(actual) {
            continue;
        }
        match kind {
            "aws_fsx_ontap_volume"
                if scalar(evidence, r.target, "volume_type") != Some("ONTAP") =>
            {
                continue;
            }
            "aws_fsx_s3_access_point_attachment"
                if scalar(evidence, r.target, "type") != Some("OPENZFS") =>
            {
                continue;
            }
            "aws_fsx_openzfs_volume"
                if r.attribute == "origin_snapshot.snapshot_arn"
                    && !matches!(
                        scalar(evidence, r.target, "origin_snapshot.copy_strategy"),
                        Some("CLONE" | "FULL_COPY" | "INCREMENTAL_COPY")
                    ) =>
            {
                continue;
            }
            "aws_fsx_backup" => {
                let other = if r.attribute == "file_system_id" {
                    "volume_id"
                } else {
                    "file_system_id"
                };
                let (value, unknown) = evidence[r.target];
                if !value[other].is_null()
                    || !matches!(&unknown[other], Value::Null | Value::Bool(false))
                {
                    continue;
                }
            }
            _ => {}
        }
        r.service_reference_verified = true;
    }
    let parents = refs
        .iter()
        .filter(|r| {
            r.service_reference_verified
                && nodes[r.target].resource_type == "aws_fsx_openzfs_volume"
                && r.attribute == "parent_volume_id"
        })
        .map(|r| (r.target, r.sources[0]))
        .collect::<BTreeMap<_, _>>();
    let unproven = parents
        .keys()
        .copied()
        .filter(|start| {
            let mut seen = BTreeSet::new();
            let mut current = *start;
            while let Some(&parent) = parents.get(&current) {
                if !seen.insert(current) {
                    return true;
                }
                current = parent;
            }
            nodes[current].resource_type != "aws_fsx_openzfs_file_system"
        })
        .collect::<BTreeSet<_>>();
    for r in refs {
        if r.attribute == "parent_volume_id" && unproven.contains(&r.target) {
            r.service_reference_verified = false;
        }
    }
}
