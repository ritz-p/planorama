use super::super::{service_evidence, service_references, streaming};
use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

fn supported(kind: &str) -> bool {
    matches!(
        kind,
        "aws_fsx_windows_file_system"
            | "aws_fsx_ontap_file_system"
            | "aws_fsx_openzfs_file_system"
            | "aws_fsx_file_cache"
    )
}

fn rule(
    kind: &str,
    path: &str,
) -> Option<(&'static [&'static str], &'static str, bool, &'static str)> {
    if !supported(kind) {
        return None;
    }
    match path {
        "subnet_ids" => Some((&["aws_subnet"], "id", true, "Configured subnets")),
        "security_group_ids" => Some((
            &["aws_security_group"],
            "id",
            true,
            "Configured security groups",
        )),
        "kms_key_id" => Some((&["aws_kms_key"], "arn", false, "Encryption key reference")),
        "preferred_subnet_id" if kind != "aws_fsx_file_cache" => {
            Some((&["aws_subnet"], "id", false, "Preferred subnet"))
        }
        "route_table_ids"
            if matches!(
                kind,
                "aws_fsx_ontap_file_system" | "aws_fsx_openzfs_file_system"
            ) =>
        {
            Some((&["aws_route_table"], "id", true, "Filesystem route tables"))
        }
        "backup_id"
            if matches!(
                kind,
                "aws_fsx_windows_file_system" | "aws_fsx_openzfs_file_system"
            ) =>
        {
            Some((&["aws_fsx_backup"], "id", false, "Source backup"))
        }
        "active_directory_id" if kind == "aws_fsx_windows_file_system" => Some((
            &["aws_directory_service_directory"],
            "id",
            false,
            "Active Directory",
        )),
        "self_managed_active_directory.domain_join_service_account_secret"
            if kind == "aws_fsx_windows_file_system" =>
        {
            Some((
                &["aws_secretsmanager_secret"],
                "arn",
                false,
                "Domain join secret reference",
            ))
        }
        "audit_log_configuration.audit_log_destination"
            if kind == "aws_fsx_windows_file_system" =>
        {
            Some((
                &[
                    "aws_cloudwatch_log_group",
                    "aws_kinesis_firehose_delivery_stream",
                ],
                "arn",
                false,
                "Audit log destination",
            ))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
}

fn scalar<'a>(evidence: &[(&'a Value, &'a Value)], target: usize, path: &str) -> Option<&'a str> {
    let (value, unknown) = evidence[target];
    streaming::values(value, unknown, path, false)?
        .into_iter()
        .next()
}

fn network_contract(kind: &str, target: usize, evidence: &[(&Value, &Value)]) -> bool {
    let (value, unknown) = evidence[target];
    let Some(subnets) = streaming::values(value, unknown, "subnet_ids", true) else {
        return false;
    };
    let count = match (kind, scalar(evidence, target, "deployment_type")) {
        ("aws_fsx_file_cache", _)
            if scalar(evidence, target, "file_cache_type") == Some("LUSTRE")
                && scalar(evidence, target, "lustre_configuration.deployment_type")
                    == Some("CACHE_1") =>
        {
            1
        }
        (
            "aws_fsx_windows_file_system"
            | "aws_fsx_openzfs_file_system"
            | "aws_fsx_ontap_file_system",
            Some("SINGLE_AZ_1" | "SINGLE_AZ_2"),
        ) => 1,
        (
            "aws_fsx_windows_file_system"
            | "aws_fsx_openzfs_file_system"
            | "aws_fsx_ontap_file_system",
            Some("MULTI_AZ_1"),
        ) => 2,
        ("aws_fsx_ontap_file_system", Some("MULTI_AZ_2")) => 2,
        _ => return false,
    };
    if subnets.len() != count {
        return false;
    }
    let preferred = scalar(evidence, target, "preferred_subnet_id");
    if (count == 2
        || kind == "aws_fsx_ontap_file_system"
        || !value["preferred_subnet_id"].is_null()
        || !matches!(
            &unknown["preferred_subnet_id"],
            Value::Null | Value::Bool(false)
        ))
        && !preferred.is_some_and(|id| subnets.contains(id))
    {
        return false;
    }
    true
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs.iter_mut() {
        let kind = nodes[r.target].resource_type.as_str();
        let Some((kinds, output, collection, _)) = rule(kind, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !service_evidence::eligible(nodes, r)
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(output)
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let Some(actual) = streaming::values(value, unknown, &r.attribute, collection) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                kinds
                    .contains(&nodes[s].resource_type.as_str())
                    .then(|| scalar(evidence, s, output))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        if !expected.is_some_and(|set| set.len() == r.sources.len() && set == actual) {
            continue;
        }
        if matches!(
            r.attribute.as_str(),
            "subnet_ids" | "security_group_ids" | "preferred_subnet_id" | "route_table_ids"
        ) {
            if !network_contract(kind, r.target, evidence) {
                continue;
            }
            let Some(vpc) = scalar(evidence, r.target, "vpc_id") else {
                continue;
            };
            if !r
                .sources
                .iter()
                .all(|&s| scalar(evidence, s, "vpc_id") == Some(vpc))
            {
                continue;
            }
        }
        if r.attribute == "audit_log_configuration.audit_log_destination" {
            let levels = [
                "audit_log_configuration.file_access_audit_log_level",
                "audit_log_configuration.file_share_access_audit_log_level",
            ]
            .map(|p| scalar(evidence, r.target, p));
            if !levels.iter().all(|l| {
                matches!(
                    l,
                    Some("DISABLED" | "SUCCESS_ONLY" | "FAILURE_ONLY" | "SUCCESS_AND_FAILURE")
                )
            }) || !levels.iter().any(|l| {
                matches!(
                    l,
                    Some("SUCCESS_ONLY" | "FAILURE_ONLY" | "SUCCESS_AND_FAILURE")
                )
            }) {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
    let valid_context = nodes
        .iter()
        .enumerate()
        .map(|(target, node)| {
            if !supported(&node.resource_type) {
                return false;
            }
            let (value, unknown) = evidence[target];
            ["preferred_subnet_id", "route_table_ids"]
                .iter()
                .all(|path| {
                    if value[*path].is_null()
                        && matches!(&unknown[*path], Value::Null | Value::Bool(false))
                    {
                        return true;
                    }
                    if *path == "route_table_ids"
                        && value[*path].as_array().is_some_and(Vec::is_empty)
                        && matches!(&unknown[*path], Value::Null | Value::Bool(false))
                    {
                        return true;
                    }
                    let mut matching = refs
                        .iter()
                        .filter(|r| r.target == target && r.attribute == *path);
                    matching
                        .next()
                        .is_some_and(|r| r.service_reference_verified)
                        && matching.next().is_none()
                })
        })
        .collect::<Vec<_>>();
    for r in refs {
        if supported(&nodes[r.target].resource_type) && r.attribute == "subnet_ids" {
            r.service_reference_verified &= valid_context[r.target];
        }
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if !supported(&node.resource_type) {
            continue;
        }
        let proof = |path: &str| {
            let mut refs = raw
                .attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == path);
            let r = refs.next()?;
            (refs.next().is_none() && service_references::verified(raw, r)).then_some(r)
        };
        let Some((subnets, groups)) = proof("subnet_ids").zip(proof("security_group_ids")) else {
            continue;
        };
        let members = subnets.sources.iter().chain(&groups.sources);
        let Some(parents) = members
            .clone()
            .map(|&s| streaming::vpc(raw, s))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().expect("one VPC");
        if proof("route_table_ids").is_some_and(|r| {
            r.sources
                .iter()
                .any(|&s| streaming::vpc(raw, s) != Some(parent))
        }) {
            continue;
        }
        if raw.attributes.iter().any(|r| {
            r.target == target
                && matches!(
                    r.attribute.as_str(),
                    "preferred_subnet_id" | "route_table_ids"
                )
                && !service_references::verified(raw, r)
        }) {
            continue;
        }
        let mut evidence = Vec::new();
        for &member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
