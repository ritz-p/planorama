use super::{Evidence, proof};
use crate::model::{AttributeReference, ConfiguredValueMatch, EntityMode, TerraformEntity};
use std::collections::BTreeMap;

fn worker_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

pub(crate) fn recover(
    nodes: &[TerraformEntity],
    references: &[AttributeReference],
    evidence: &[Evidence<'_>],
) -> Vec<ConfiguredValueMatch> {
    let mut zones = BTreeMap::<&str, Vec<usize>>::new();
    let mut workers = BTreeMap::<(&str, &str), Vec<usize>>::new();
    for (index, (node, value)) in nodes.iter().zip(evidence).enumerate() {
        if !proof::current(node) {
            continue;
        }
        if node.resource_type == "cloudflare_zone" {
            if let Ok(id) = value.id("id") {
                zones.entry(id).or_default().push(index);
            }
        }
        if !matches!(
            node.resource_type.as_str(),
            "cloudflare_worker" | "cloudflare_workers_script"
        ) {
            continue;
        }
        if let (Ok(account), Ok(name)) = (
            value.id("account_id"),
            value.text(if node.resource_type == "cloudflare_worker" {
                "name"
            } else {
                "script_name"
            }),
        ) {
            if worker_name(name) {
                workers.entry((account, name)).or_default().push(index);
            }
        }
    }
    let explicit: std::collections::BTreeSet<_> = references
        .iter()
        .filter(|r| r.has_references)
        .map(|r| (r.target, r.attribute.as_str()))
        .collect();
    let mut result = Vec::new();
    for (consumer, (node, value)) in nodes.iter().zip(evidence).enumerate() {
        if !proof::current(node) || node.mode != EntityMode::Managed {
            continue;
        }
        let worker_field = match node.resource_type.as_str() {
            "cloudflare_workers_route" => "script",
            "cloudflare_workers_custom_domain" => "service",
            _ => continue,
        };
        let Ok(zone) = value.id("zone_id") else {
            continue;
        };
        let Some([zone_source]) = zones.get(zone).map(Vec::as_slice) else {
            continue;
        };
        if !proof::known_configuration_matches(node, &nodes[*zone_source]) {
            continue;
        }
        let zone_value = evidence[*zone_source];
        let Ok(account) = zone_value.id("account.id") else {
            continue;
        };
        if nodes[*zone_source].mode == EntityMode::Data && zone_value.id("zone_id") != Ok(zone) {
            continue;
        }
        if node.resource_type == "cloudflare_workers_custom_domain"
            && value.id("account_id") != Ok(account)
        {
            continue;
        }
        let eligible = |attribute: &str| !explicit.contains(&(consumer, attribute));
        if eligible("zone_id") {
            result.push(ConfiguredValueMatch {
                consumer,
                source: *zone_source,
                attribute: "zone_id",
            });
        }
        if value
            .at("environment")
            .and_then(Evidence::require_null)
            .is_err()
            || !eligible(worker_field)
        {
            continue;
        }
        let Ok(name) = value.text(worker_field) else {
            continue;
        };
        if !worker_name(name) {
            continue;
        }
        let Some([source]) = workers.get(&(account, name)).map(Vec::as_slice) else {
            continue;
        };
        let worker = &nodes[*source];
        if !proof::known_configuration_matches(node, worker) {
            continue;
        }
        let worker_value = evidence[*source];
        if worker.resource_type == "cloudflare_workers_script"
            && worker_value.text("id") != Ok(name)
        {
            continue;
        }
        if worker_value
            .at("environment")
            .and_then(Evidence::require_null)
            .is_err()
        {
            continue;
        }
        result.push(ConfiguredValueMatch {
            consumer,
            source: *source,
            attribute: worker_field,
        });
    }
    result
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_recovery.rs"]
mod tests;
