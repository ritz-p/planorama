use crate::model::{
    EntityMode, Placement, ResourceRole, TerraformEntity, architecture::CloudflareScope,
};
use serde_json::Value;
mod types;

pub(crate) fn known(resource: &str, mode: EntityMode) -> bool {
    types::TYPES
        .iter()
        .any(|&(name, kind, _, _)| name == resource && kind == mode)
}

pub(crate) fn classify(resource: &str, mode: EntityMode) -> ResourceRole {
    if known(resource, mode) {
        ResourceRole::Node
    } else {
        ResourceRole::Unknown
    }
}

pub(crate) fn placement(resource: &str, mode: EntityMode) -> Placement {
    if !known(resource, mode) {
        Placement::Unresolved
    } else if mode == EntityMode::Managed
        && matches!(
            resource,
            "cloudflare_account"
                | "cloudflare_zone"
                | "cloudflare_load_balancer"
                | "cloudflare_workers_script"
                | "cloudflare_pages_project"
                | "cloudflare_r2_bucket"
                | "cloudflare_workers_kv_namespace"
                | "cloudflare_d1_database"
                | "cloudflare_zero_trust_tunnel_cloudflared"
                | "cloudflare_zero_trust_access_application"
        )
    {
        Placement::Spatial
    } else {
        Placement::Logical
    }
}

pub(crate) fn service_name(resource: &str, mode: EntityMode) -> Option<&'static str> {
    known(resource, mode).then_some("Cloudflare")
}

fn known_id(value: &Value, mask: &Value, path: &[&str]) -> Option<String> {
    if mask == &Value::Bool(true) {
        return None;
    }
    if let Some((first, rest)) = path.split_first() {
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        return known_id(&value[*first], &mask[*first], rest);
    }
    if !matches!(mask, Value::Null | Value::Bool(false)) {
        return None;
    }
    value
        .as_str()
        .filter(|v| v.len() == 32 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

pub(crate) fn annotate(nodes: &mut [TerraformEntity], evidence: &[(&Value, &Value)]) {
    for (node, &(value, mask)) in nodes.iter_mut().zip(evidence) {
        if !node.provider.is_cloudflare() || !known(&node.resource_type, node.mode) {
            continue;
        }
        let Some((_, _, account_path, zone_path)) = types::TYPES
            .iter()
            .find(|&&(name, mode, _, _)| name == node.resource_type && mode == node.mode)
        else {
            continue;
        };
        if account_path.is_empty() && zone_path.is_empty() {
            continue;
        }
        node.cloudflare_scope = Some(CloudflareScope {
            account: known_id(value, mask, account_path),
            zone: known_id(value, mask, zone_path),
        });
    }
}

#[cfg(test)]
#[path = "../../tests/unit/cloudflare.rs"]
mod tests;
