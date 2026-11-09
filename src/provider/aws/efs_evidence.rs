use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/efs_evidence.rs"]
mod tests;

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    if !r.attribute.starts_with("destination.") {
        return super::apprunner::bindings(raw, r);
    }
    let account = raw.nodes[r.target]
        .provider_configuration
        .as_ref()
        .and_then(|c| c.account.as_ref());
    if account.is_none()
        || r.sources.iter().any(|&s| {
            raw.nodes[s]
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref())
                != account
        })
    {
        return false;
    }
    std::iter::once(r.target)
        .chain(r.sources.iter().copied())
        .all(|index| {
            let node = &raw.nodes[index];
            node.provider_configuration.as_ref().is_some_and(|c| {
                !c.account_conflict
                    && node
                        .entity
                        .scope
                        .as_ref()
                        .is_some_and(|s| s.region == c.region)
            })
        })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_efs_file_system", "kms_key_id") => Some("Encryption key reference"),
        ("aws_efs_backup_policy" | "aws_efs_file_system_policy", "file_system_id") => {
            Some("File system")
        }
        ("aws_efs_replication_configuration", "source_file_system_id") => {
            Some("Replication source")
        }
        ("aws_efs_replication_configuration", "destination.file_system_id") => {
            Some("Replication destination")
        }
        ("aws_efs_replication_configuration", "destination.kms_key_id") => {
            Some("Destination encryption key")
        }
        ("aws_efs_mount_target", "file_system_id") => Some("File system"),
        ("aws_efs_mount_target", "subnet_id") => Some("Mount target subnet"),
        ("aws_efs_mount_target", "security_groups") => Some("Configured security groups"),
        _ => None,
    }
}

fn scalar<'a>(evidence: &[(&'a Value, &'a Value)], index: usize, path: &str) -> Option<&'a str> {
    let (v, m) = evidence[index];
    super::streaming::values(v, m, path, false)?
        .into_iter()
        .next()
}

fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[r.target];
    let collection = r.attribute == "security_groups";
    if !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
    {
        return false;
    }
    let destination = r.attribute.starts_with("destination.");
    let (v, m) = evidence[r.target];
    if destination && v["destination"].as_array().is_none_or(|a| a.len() != 1) {
        return false;
    }
    let Some(actual) = super::streaming::values(v, m, &r.attribute, collection) else {
        return false;
    };
    if node.resource_type == "aws_efs_file_system"
        && (v["encrypted"] != Value::Bool(true)
            || !matches!(&m["encrypted"], Value::Null | Value::Bool(false)))
    {
        return false;
    }
    let output = r.source_attributes.first().unwrap().as_str();
    let expected = r
        .sources
        .iter()
        .map(|&s| {
            let endpoint = &nodes[s];
            let current = |n: &TerraformEntity| {
                n.provider.is_aws()
                    && n.mode == EntityMode::Managed
                    && n.action != Action::Delete
                    && n.deposed_key.is_none()
            };
            if s == r.target || !current(node) || !current(endpoint) {
                return None;
            }
            let valid = matches!(
                (
                    r.attribute.as_str(),
                    endpoint.resource_type.as_str(),
                    output,
                ),
                (
                    "file_system_id" | "source_file_system_id" | "destination.file_system_id",
                    "aws_efs_file_system",
                    "id",
                ) | ("subnet_id", "aws_subnet", "id")
                    | ("security_groups", "aws_security_group", "id")
                    | ("kms_key_id", "aws_kms_key", "arn")
                    | (
                        "destination.kms_key_id",
                        "aws_kms_key",
                        "id" | "key_id" | "arn"
                    )
                    | ("destination.kms_key_id", "aws_kms_alias", "name" | "arn")
            );
            if !valid {
                return None;
            }
            let (Some(a), Some(b)) = (
                &node.provider_configuration,
                &endpoint.provider_configuration,
            ) else {
                return None;
            };
            if destination {
                if a.account_conflict
                    || b.account_conflict
                    || a.account != b.account
                    || a.region.is_none()
                    || b.region.is_none()
                    || scalar(evidence, r.target, "destination.region")
                        != b.region.as_ref().map(|r| r.0.as_str())
                {
                    return None;
                }
            } else if !super::service_evidence::eligible(nodes, r) {
                return None;
            }
            scalar(evidence, s, output)
        })
        .collect::<Option<BTreeSet<_>>>();
    expected.is_some_and(|e| e.len() == r.sources.len() && e == actual)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let unique = |r: &AttributeReference| {
        refs.iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            == 1
    };
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let mut valid = unique(r) && verify(nodes, r, evidence);
            if valid && r.attribute == "security_groups" {
                let subnet = refs.iter().find(|s| {
                    s.target == r.target
                        && s.attribute == "subnet_id"
                        && unique(s)
                        && verify(nodes, s, evidence)
                });
                valid = subnet
                    .and_then(|s| scalar(evidence, s.sources[0], "vpc_id"))
                    .is_some_and(|vpc| {
                        r.sources
                            .iter()
                            .all(|&s| scalar(evidence, s, "vpc_id") == Some(vpc))
                    });
            }
            Some(valid)
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
        }
    }
}
