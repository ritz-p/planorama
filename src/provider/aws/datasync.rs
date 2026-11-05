use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/datasync.rs"]
mod tests;

fn rule(resource: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, path) {
        ("aws_datasync_task", "source_location_arn") => {
            Some(("location", "arn", false, "Source location"))
        }
        ("aws_datasync_task", "destination_location_arn") => {
            Some(("location", "arn", false, "Destination location"))
        }
        ("aws_datasync_task", "cloudwatch_log_group_arn") => {
            Some(("aws_cloudwatch_log_group", "arn", false, "Task log group"))
        }
        ("aws_datasync_task", "task_report_config.s3_destination.s3_bucket_arn") => {
            Some(("aws_s3_bucket", "arn", false, "Task report bucket"))
        }
        ("aws_datasync_task", "task_report_config.s3_destination.bucket_access_role_arn") => {
            Some(("aws_iam_role", "arn", false, "Task report access role"))
        }
        ("aws_datasync_agent", "subnet_arns") => {
            Some(("aws_subnet", "arn", true, "Task interface subnets"))
        }
        ("aws_datasync_agent", "vpc_endpoint_id") => {
            Some(("aws_vpc_endpoint", "id", false, "DataSync VPC endpoint"))
        }
        ("aws_datasync_location_efs", "ec2_config.subnet_arn") => {
            Some(("aws_subnet", "arn", false, "Connection subnet"))
        }
        (
            "aws_datasync_agent"
            | "aws_datasync_location_fsx_lustre_file_system"
            | "aws_datasync_location_fsx_openzfs_file_system"
            | "aws_datasync_location_fsx_windows_file_system"
            | "aws_datasync_location_fsx_ontap_file_system",
            "security_group_arns",
        )
        | ("aws_datasync_location_efs", "ec2_config.security_group_arns") => Some((
            "aws_security_group",
            "arn",
            true,
            "Connection security groups",
        )),
        ("aws_datasync_location_efs", "efs_file_system_arn") => {
            Some(("aws_efs_file_system", "arn", false, "EFS file system"))
        }
        ("aws_datasync_location_efs", "access_point_arn") => {
            Some(("aws_efs_access_point", "arn", false, "EFS access point"))
        }
        ("aws_datasync_location_efs", "file_system_access_role_arn")
        | ("aws_datasync_location_s3", "s3_config.bucket_access_role_arn") => {
            Some(("aws_iam_role", "arn", false, "Storage access role"))
        }
        ("aws_datasync_location_fsx_lustre_file_system", "fsx_filesystem_arn") => Some((
            "aws_fsx_lustre_file_system",
            "arn",
            false,
            "Lustre file system",
        )),
        ("aws_datasync_location_fsx_openzfs_file_system", "fsx_filesystem_arn") => Some((
            "aws_fsx_openzfs_file_system",
            "arn",
            false,
            "OpenZFS file system",
        )),
        ("aws_datasync_location_fsx_windows_file_system", "fsx_filesystem_arn") => Some((
            "aws_fsx_windows_file_system",
            "arn",
            false,
            "Windows file system",
        )),
        ("aws_datasync_location_fsx_ontap_file_system", "storage_virtual_machine_arn") => Some((
            "aws_fsx_ontap_storage_virtual_machine",
            "arn",
            false,
            "ONTAP storage virtual machine",
        )),
        ("aws_datasync_location_s3", "s3_bucket_arn") => {
            Some(("aws_s3_bucket", "arn", false, "S3 storage endpoint"))
        }
        (
            "aws_datasync_location_s3"
            | "aws_datasync_location_azure_blob"
            | "aws_datasync_location_hdfs"
            | "aws_datasync_location_object_storage"
            | "aws_datasync_location_smb",
            "agent_arns",
        )
        | ("aws_datasync_location_nfs", "on_prem_config.agent_arns") => {
            Some(("aws_datasync_agent", "arn", true, "DataSync agents"))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, path: &str) -> Option<&'static str> {
    rule(resource, path).map(|r| r.3)
}

fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[r.target];
    let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
        return false;
    };
    if !super::service_evidence::eligible_with_global_role(nodes, r, true)
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(field)
    {
        return false;
    }
    let (value, unknown) = evidence[r.target];
    let mut block = value;
    let parts = r.attribute.split('.').collect::<Vec<_>>();
    for name in &parts[..parts.len() - 1] {
        let Some(items) = block[*name].as_array() else {
            return false;
        };
        let [item] = items.as_slice() else {
            return false;
        };
        block = item;
    }
    if r.attribute == "vpc_endpoint_id"
        && super::service_evidence::known(evidence, r.sources[0], "vpc_endpoint_type")
            != Some("Interface")
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, unknown, &r.attribute, collection) else {
        return false;
    };
    let expected = r
        .sources
        .iter()
        .map(|&source| {
            let endpoint = &nodes[source];
            let output = super::service_evidence::known(evidence, source, field)?;
            let compatible = if kind == "location" {
                matches!(
                    endpoint.resource_type.as_str(),
                    "aws_datasync_location_azure_blob"
                        | "aws_datasync_location_efs"
                        | "aws_datasync_location_fsx_lustre_file_system"
                        | "aws_datasync_location_fsx_ontap_file_system"
                        | "aws_datasync_location_fsx_openzfs_file_system"
                        | "aws_datasync_location_fsx_windows_file_system"
                        | "aws_datasync_location_hdfs"
                        | "aws_datasync_location_nfs"
                        | "aws_datasync_location_object_storage"
                        | "aws_datasync_location_s3"
                        | "aws_datasync_location_smb"
                )
            } else if node.resource_type == "aws_datasync_location_s3"
                && r.attribute == "s3_bucket_arn"
            {
                let parts = output.splitn(6, ':').collect::<Vec<_>>();
                parts.len() == 6
                    && parts[0] == "arn"
                    && match parts[2] {
                        "s3" => {
                            endpoint.resource_type == "aws_s3_bucket"
                                && parts[3].is_empty()
                                && parts[4].is_empty()
                                && !parts[5].contains('/')
                        }
                        "s3-outposts" => {
                            endpoint.resource_type == "aws_s3_access_point"
                                && parts[5].starts_with("outpost/")
                                && parts[5].contains("/accesspoint/")
                        }
                        _ => false,
                    }
            } else {
                endpoint.resource_type == kind
            };
            compatible.then_some(output)
        })
        .collect::<Option<BTreeSet<_>>>();
    expected.is_some_and(|e| e.len() == r.sources.len() && e == actual)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let proof = |target, path: &str| {
        let mut candidates = refs
            .iter()
            .filter(|r| r.target == target && r.attribute == path);
        let r = candidates.next()?;
        (candidates.next().is_none() && verify(nodes, r, evidence)).then(|| r.sources[0])
    };
    let results = refs
        .iter()
        .enumerate()
        .filter(|(_, r)| label(&nodes[r.target].resource_type, &r.attribute).is_some())
        .map(|(i, r)| {
            let mut valid = verify(nodes, r, evidence);
            if valid
                && nodes[r.target].resource_type == "aws_datasync_location_s3"
                && r.attribute == "agent_arns"
            {
                valid = proof(r.target, "s3_bucket_arn")
                    .is_some_and(|endpoint| nodes[endpoint].resource_type == "aws_s3_access_point");
            }
            if valid && r.attribute == "access_point_arn" {
                valid = proof(r.target, "efs_file_system_arn").is_some_and(|fs| {
                    let fs_id = super::service_evidence::known(evidence, fs, "id");
                    fs_id.is_some()
                        && fs_id
                            == super::service_evidence::known(
                                evidence,
                                r.sources[0],
                                "file_system_id",
                            )
                }) && super::service_evidence::known(
                    evidence,
                    r.target,
                    "in_transit_encryption",
                ) == Some("TLS1_2");
            }
            if valid && r.attribute == "storage_virtual_machine_arn" {
                let fs_id =
                    super::service_evidence::known(evidence, r.sources[0], "file_system_id");
                valid = super::service_evidence::known(evidence, r.target, "fsx_filesystem_arn")
                    .is_some_and(|arn| {
                        arn.split_once(":file-system/").is_some_and(|(prefix, id)| {
                            Some(id) == fs_id
                                && super::service_evidence::known(evidence, r.sources[0], "arn")
                                    .and_then(|svm| svm.split_once(":storage-virtual-machine/"))
                                    .is_some_and(|(svm_prefix, _)| prefix == svm_prefix)
                        })
                    });
            }
            (i, valid)
        })
        .collect::<Vec<_>>();
    for (i, valid) in results {
        refs[i].service_reference_verified = valid;
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&source| {
        let node = &raw.nodes[r.target];
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
            endpoint.resource_type == "aws_iam_role",
        )
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        let paths = match node.resource_type.as_str() {
            "aws_datasync_agent" => ["subnet_arns", "security_group_arns"],
            "aws_datasync_location_efs" => {
                ["ec2_config.subnet_arn", "ec2_config.security_group_arns"]
            }
            _ => continue,
        };
        let mut members = Vec::new();
        for path in paths {
            let mut refs = raw
                .attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == path);
            if let Some(r) = refs.next() {
                if refs.next().is_none() && super::service_references::verified(raw, r) {
                    members.extend(&r.sources);
                } else {
                    members.clear();
                    break;
                }
            } else {
                members.clear();
                break;
            }
        }
        let Some(parents) = members
            .iter()
            .map(|&i| super::streaming::vpc(raw, i))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().expect("one VPC");
        let evidence = members
            .iter()
            .flat_map(|&member| {
                [
                    relationships::reference(graph, parent, member),
                    relationships::reference(graph, member, target),
                ]
            })
            .collect();
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
