use crate::model::{
    EntityMode, Placement, ResourceRole, TerraformEntity, architecture::CloudflareScope,
};
use serde_json::Value;
mod administration;
mod applications;
mod edge;
mod proof;
mod types;
mod zero_trust;

pub(crate) fn known(resource: &str, mode: EntityMode) -> bool {
    types::TYPES
        .iter()
        .any(|&(name, kind, _, _)| name == resource && kind == mode)
}

pub(crate) fn classify(resource: &str, mode: EntityMode) -> ResourceRole {
    if known(resource, mode) {
        if mode == EntityMode::Managed
            && matches!(
                resource,
                "cloudflare_ruleset"
                    | "cloudflare_page_rule"
                    | "cloudflare_zero_trust_access_policy"
            )
        {
            return ResourceRole::Policy;
        }
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
                | "cloudflare_worker"
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
    known(resource, mode).then_some(match resource {
        "cloudflare_zone" => "Cloudflare Zone",
        "cloudflare_dns_record" => "Cloudflare DNS",
        "cloudflare_zone_dnssec" => "Cloudflare DNSSEC",
        "cloudflare_load_balancer"
        | "cloudflare_load_balancer_pool"
        | "cloudflare_load_balancer_monitor"
        | "cloudflare_load_balancer_monitor_group" => "Cloudflare Load Balancing",
        "cloudflare_ruleset" => "Cloudflare Rulesets",
        "cloudflare_page_rule" => "Cloudflare Page Rules",
        "cloudflare_list" => "Cloudflare Lists",
        "cloudflare_worker"
        | "cloudflare_worker_version"
        | "cloudflare_workers_script"
        | "cloudflare_workers_deployment"
        | "cloudflare_workers_route"
        | "cloudflare_workers_custom_domain" => "Cloudflare Workers",
        "cloudflare_pages_project" | "cloudflare_pages_domain" => "Cloudflare Pages",
        "cloudflare_r2_bucket" => "Cloudflare R2",
        "cloudflare_workers_kv_namespace" => "Cloudflare KV",
        "cloudflare_d1_database" => "Cloudflare D1",
        "cloudflare_zero_trust_access_application"
        | "cloudflare_zero_trust_access_policy"
        | "cloudflare_zero_trust_access_identity_provider" => "Cloudflare Access",
        "cloudflare_zero_trust_tunnel_cloudflared"
        | "cloudflare_zero_trust_tunnel_cloudflared_config"
        | "cloudflare_zero_trust_tunnel_cloudflared_route"
        | "cloudflare_zero_trust_tunnel_cloudflared_virtual_network" => "Cloudflare Tunnel",
        _ => "Cloudflare",
    })
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
        .map(str::to_owned)
}

pub(crate) fn annotate(
    nodes: &mut [TerraformEntity],
    references: &mut [crate::model::AttributeReference],
    evidence: &[(&Value, &Value)],
) {
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
    administration::annotate(nodes, references, evidence);
    edge::annotate(nodes, references, evidence);
    applications::annotate(nodes, references, evidence);
    zero_trust::annotate(nodes, references, evidence);
}

#[cfg(test)]
#[path = "../../tests/unit/cloudflare.rs"]
mod tests;

pub(crate) fn facts(plan: &crate::model::TerraformPlan, graph: &mut crate::model::Graph) {
    for r in &plan.attributes {
        let node = &mut graph.nodes[r.target];
        if !node.provider.is_cloudflare() || !r.has_references {
            continue;
        }
        let Some(label) = administration::label(&node.resource_type, &r.attribute)
            .or_else(|| edge::label(&node.resource_type, &r.attribute))
            .or_else(|| applications::label(&node.resource_type, &r.attribute))
            .or_else(|| zero_trust::label(&node.resource_type, &r.attribute))
        else {
            continue;
        };
        for reference in node
            .references
            .iter_mut()
            .filter(|reference| !reference.dependency && reference.attribute == r.attribute)
        {
            reference.label = label;
            reference.kind = if administration::label(&node.resource_type, &r.attribute).is_some() {
                crate::model::ReferenceKind::Membership
            } else {
                crate::model::ReferenceKind::Configured
            };
            reference.show_spatial = true;
            reference.complete = r.service_reference_verified;
            if !reference.complete {
                reference.targets.clear();
            }
        }
    }
}
