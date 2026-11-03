use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/backup_vaults.rs"]
mod tests;

pub(super) fn helper(kind: &str) -> bool {
    matches!(
        kind,
        "aws_backup_vault_lock_configuration"
            | "aws_backup_vault_notifications"
            | "aws_backup_vault_policy"
    )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if helper(kind) && path == "backup_vault_name" {
        return Some("Backup vault owner");
    }
    match (kind, path) {
        ("aws_backup_logically_air_gapped_vault", "encryption_key_arn") => {
            Some("Encryption key reference")
        }
        ("aws_backup_vault_notifications", "sns_topic_arn") => Some("Vault notification topic"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !super::service_evidence::eligible(nodes, r)
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let source = r.sources[0];
        let output = r.source_attributes.first().unwrap().as_str();
        let valid = match (r.attribute.as_str(), nodes[source].resource_type.as_str()) {
            ("backup_vault_name", "aws_backup_vault" | "aws_backup_logically_air_gapped_vault") => {
                matches!(output, "name" | "id")
            }
            ("sns_topic_arn", "aws_sns_topic") | ("encryption_key_arn", "aws_kms_key") => {
                output == "arn"
            }
            _ => false,
        };
        if !valid {
            continue;
        }
        let Some(actual) = super::service_evidence::known(evidence, r.target, &r.attribute) else {
            continue;
        };
        if super::service_evidence::known(evidence, source, output) != Some(actual) {
            continue;
        }
        if r.attribute == "backup_vault_name"
            && super::service_evidence::known(evidence, r.target, "backup_vault_arn").is_some_and(
                |arn| super::service_evidence::known(evidence, source, "arn") != Some(arn),
            )
        {
            continue;
        }
        r.service_reference_verified = true;
    }
}
