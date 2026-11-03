use crate::model::{Action, AttributeReference, AwsAccountScope, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/backup_plan.rs"]
mod tests;

const COPY: &str = "rule.copy_action.destination_vault_arn";
const ROLE: &str = "scan_setting.scanner_role_arn";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if kind != "aws_backup_plan" {
        return None;
    }
    match path {
        "rule.target_vault_name" => Some("Backup vault reference"),
        "rule.target_logically_air_gapped_backup_vault_arn" => Some("Air-gapped backup vault"),
        COPY => Some("Copy destination vault"),
        ROLE => Some("Backup scanner role"),
        _ => None,
    }
}

fn values<'a>(v: &'a Value, m: &Value, path: &[&str], found: &mut BTreeSet<&'a str>) -> Option<()> {
    if let Some(items) = v.as_array() {
        if items.is_empty()
            || !matches!(m, Value::Null | Value::Bool(false) | Value::Array(_))
            || m.as_array().is_some_and(|a| a.len() != items.len())
        {
            return None;
        }
        for (i, item) in items.iter().enumerate() {
            values(item, &m[i], path, found)?;
        }
    } else if let Some((field, remaining)) = path.split_first() {
        if !v.is_object() || !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        values(&v[*field], &m[*field], remaining, found)?;
    } else {
        if !matches!(m, Value::Null | Value::Bool(false)) {
            return None;
        }
        found.insert(v.as_str().filter(|s| !s.is_empty())?);
    }
    Some(())
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
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        let current = |n: &TerraformEntity| {
            n.provider.is_aws()
                && n.mode == EntityMode::Managed
                && n.action != Action::Delete
                && n.deposed_key.is_none()
        };
        if counts[&(r.target, r.attribute.clone())] != 1
            || !current(&nodes[r.target])
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
        {
            continue;
        }
        if r.attribute != COPY
            && !super::service_evidence::eligible_with_global_role(nodes, r, true)
        {
            continue;
        }
        let (v, m) = evidence[r.target];
        let mut actual = BTreeSet::new();
        if values(
            v,
            m,
            &r.attribute.split('.').collect::<Vec<_>>(),
            &mut actual,
        )
        .is_none()
        {
            continue;
        }
        if r.attribute == ROLE {
            let mut scanners = BTreeSet::new();
            if values(v, m, &["scan_setting", "malware_scanner"], &mut scanners).is_none()
                || scanners != BTreeSet::from(["GUARDDUTY"])
            {
                continue;
            }
        }
        let mut accounts = BTreeSet::new();
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                let node = &nodes[s];
                if !current(node) {
                    return None;
                }
                let outputs = r.source_attributes_by_instance.get(&s)?;
                if outputs.len() != 1 {
                    return None;
                }
                let output = outputs.first()?.as_str();
                let valid = match (r.attribute.as_str(), node.resource_type.as_str()) {
                    ("rule.target_vault_name", "aws_backup_vault") => {
                        matches!(output, "name" | "id")
                    }
                    (
                        "rule.target_logically_air_gapped_backup_vault_arn",
                        "aws_backup_logically_air_gapped_vault",
                    ) => output == "arn",
                    (COPY, "aws_backup_vault" | "aws_backup_logically_air_gapped_vault")
                    | (ROLE, "aws_iam_role") => output == "arn",
                    _ => false,
                };
                if !valid {
                    return None;
                }
                let value = super::service_evidence::known(evidence, s, output)?;
                if r.attribute == COPY {
                    let arn = value.splitn(6, ':').collect::<Vec<_>>();
                    let config = node.provider_configuration.as_ref()?;
                    if arn.len() != 6
                        || arn[0] != "arn"
                        || arn[2] != "backup"
                        || !arn[5]
                            .strip_prefix("backup-vault:")
                            .is_some_and(|name| !name.is_empty() && !name.contains('*'))
                        || config.account_conflict
                        || config.region.as_ref().map(|r| r.0.as_str()) != Some(arn[3])
                    {
                        return None;
                    }
                    accounts.insert(AwsAccountScope::new(arn[4])?);
                }
                Some(value)
            })
            .collect::<Option<BTreeSet<_>>>();
        if !expected.is_some_and(|expected| expected.len() == r.sources.len() && expected == actual)
        {
            continue;
        }
        if r.attribute == COPY {
            if accounts.len() != 1 {
                continue;
            }
            r.service_reference_owner_account = accounts.into_iter().next();
        }
        r.service_reference_verified = true;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    if r.attribute != COPY {
        return r.sources.iter().all(|&s| {
            let endpoint = &raw.nodes[s];
            super::eventbridge::bindings(
                node.provider_configuration.as_ref(),
                endpoint.provider_configuration.as_ref(),
                node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                endpoint
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                r.attribute == ROLE,
            )
        });
    }
    let Some(a) = &node.provider_configuration else {
        return false;
    };
    if a.account_conflict
        || a.region.is_none()
        || node.entity.scope.as_ref().and_then(|s| s.region.as_ref()) != a.region.as_ref()
    {
        return false;
    }
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        let Some(b) = &endpoint.provider_configuration else {
            return false;
        };
        if b.account_conflict
            || b.region.is_none()
            || endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref())
                != b.region.as_ref()
        {
            return false;
        }
        match (&a.account, &b.account) {
            (Some(left), Some(right)) => {
                left == right && Some(right) == r.service_reference_owner_account.as_ref()
            }
            (None, None) => a == b,
            _ => false,
        }
    })
}
