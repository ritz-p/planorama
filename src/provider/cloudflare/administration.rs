use crate::model::{Action, AttributeReference, TerraformEntity};
use serde_json::Value;

fn contract(kind: &str, path: &str) -> Option<(&'static [&'static str], &'static str)> {
    match (kind, path) {
        ("cloudflare_zone", "account" | "account.id") => {
            Some((&["account", "id"], "cloudflare_account"))
        }
        (
            "cloudflare_dns_record"
            | "cloudflare_zone_dnssec"
            | "cloudflare_load_balancer"
            | "cloudflare_ruleset"
            | "cloudflare_page_rule"
            | "cloudflare_workers_route"
            | "cloudflare_zero_trust_access_application"
            | "cloudflare_zero_trust_access_identity_provider",
            "zone_id",
        ) => Some((&["zone_id"], "cloudflare_zone")),
        (
            "cloudflare_load_balancer_pool"
            | "cloudflare_load_balancer_monitor"
            | "cloudflare_load_balancer_monitor_group"
            | "cloudflare_ruleset"
            | "cloudflare_list"
            | "cloudflare_worker"
            | "cloudflare_worker_version"
            | "cloudflare_workers_script"
            | "cloudflare_workers_deployment"
            | "cloudflare_workers_custom_domain"
            | "cloudflare_pages_project"
            | "cloudflare_pages_domain"
            | "cloudflare_r2_bucket"
            | "cloudflare_workers_kv_namespace"
            | "cloudflare_d1_database"
            | "cloudflare_zero_trust_access_application"
            | "cloudflare_zero_trust_access_policy"
            | "cloudflare_zero_trust_access_identity_provider"
            | "cloudflare_zero_trust_tunnel_cloudflared"
            | "cloudflare_zero_trust_tunnel_cloudflared_config"
            | "cloudflare_zero_trust_tunnel_cloudflared_route"
            | "cloudflare_zero_trust_tunnel_cloudflared_virtual_network",
            "account_id",
        ) => Some((&["account_id"], "cloudflare_account")),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    contract(kind, path).map(|(_, kind)| {
        if kind == "cloudflare_zone" {
            "Zone administration"
        } else {
            "Account administration"
        }
    })
}

fn scope_valid(node: &TerraformEntity, evidence: (&Value, &Value)) -> bool {
    if !matches!(
        node.resource_type.as_str(),
        "cloudflare_ruleset"
            | "cloudflare_zero_trust_access_application"
            | "cloudflare_zero_trust_access_identity_provider"
    ) {
        return true;
    }
    let (value, mask) = evidence;
    let presence = |field| {
        let (v, m) = super::proof::at(value, mask, field)?;
        if !matches!(m, Value::Null | Value::Bool(false)) {
            return None;
        }
        if v.is_null() {
            Some(false)
        } else {
            super::known_id(value, mask, &[field]).map(|_| true)
        }
    };
    presence("account_id")
        .zip(presence("zone_id"))
        .is_some_and(|(a, z)| a != z)
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_cloudflare()
        && node.deposed_key.is_none()
        && node.action != Action::Delete
        && node.metadata.state_removal.is_none()
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<usize> {
    let consumer = &nodes[r.target];
    if !current(consumer)
        || !scope_valid(consumer, evidence[r.target])
        || (consumer.resource_type == "cloudflare_zone"
            && consumer.mode != crate::model::EntityMode::Managed)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || refs
            .iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            != 1
    {
        return None;
    }
    let [source] = r.sources.as_slice() else {
        return None;
    };
    let endpoint = &nodes[*source];
    let (path, kind) = contract(&consumer.resource_type, &r.attribute)?;
    if !current(endpoint) || endpoint.resource_type != kind || *source == r.target {
        return None;
    }
    let attributes = r.source_attributes_by_instance.get(source)?;
    if attributes.len() != 1 {
        return None;
    }
    let attribute = attributes.first()?.as_str();
    if attribute != "id"
        && !(attribute == "zone_id"
            && endpoint.mode == crate::model::EntityMode::Data
            && kind == "cloudflare_zone")
    {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let expected = super::known_id(value, mask, path)?;
    let (value, mask) = evidence[*source];
    if super::known_id(value, mask, &[attribute])? != expected
        || super::known_id(value, mask, &["id"])? != expected
        || (kind == "cloudflare_zone"
            && endpoint.mode == crate::model::EntityMode::Data
            && super::known_id(value, mask, &["zone_id"])? != expected)
    {
        return None;
    }
    let candidates = nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            let (value, mask) = evidence[*index];
            current(node)
                && node.resource_type == kind
                && super::known_id(value, mask, &["id"]).as_ref() == Some(&expected)
        })
        .count();
    (candidates == 1).then_some(*source)
}

pub(super) fn annotate(
    nodes: &mut [TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for (node, &values) in nodes.iter_mut().zip(evidence) {
        if node.provider.is_cloudflare() && !scope_valid(node, values) {
            node.cloudflare_scope = None;
        }
    }
    for index in 0..refs.len() {
        let r = &refs[index];
        if !nodes[r.target].provider.is_cloudflare()
            || label(&nodes[r.target].resource_type, &r.attribute).is_none()
        {
            continue;
        }
        let source = verify(nodes, refs, r, evidence);
        refs[index].service_reference_verified = source.is_some();
        if refs[index].attribute == "zone_id" {
            if let Some(source) = source {
                let account = nodes[source]
                    .cloudflare_scope
                    .as_ref()
                    .and_then(|s| s.account.clone());
                if let Some(scope) = &mut nodes[refs[index].target].cloudflare_scope {
                    scope.account = account;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_dns.rs"]
mod tests;
