use super::{
    Evidence,
    evidence::{Result, collect},
    proof,
};
use crate::model::{Action, AttributeReference, CloudflareFailure as Failure, TerraformEntity};

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

fn scope_valid(node: &TerraformEntity, evidence: Evidence<'_>) -> Result<()> {
    if !matches!(
        node.resource_type.as_str(),
        "cloudflare_ruleset"
            | "cloudflare_zero_trust_access_application"
            | "cloudflare_zero_trust_access_identity_provider"
    ) {
        return Ok(());
    }
    let presence = |field| {
        let value = evidence.at(field)?;
        value.scalar()?;
        if value.value.is_null() {
            Ok(false)
        } else {
            evidence.id(field).map(|_| true)
        }
    };
    let presence = collect([presence("account_id"), presence("zone_id")])?;
    (presence[0] != presence[1])
        .then_some(())
        .ok_or(Failure::ScopeUnavailable)
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
    evidence: &[Evidence<'_>],
) -> Result<usize> {
    let consumer = &nodes[r.target];
    proof::explicit(nodes, refs, r)?;
    scope_valid(consumer, evidence[r.target])?;
    if consumer.resource_type == "cloudflare_zone"
        && consumer.mode != crate::model::EntityMode::Managed
    {
        return Err(Failure::UnsupportedContract);
    }
    let [source] = r.sources.as_slice() else {
        return Err(Failure::IncompleteProvenance);
    };
    let endpoint = &nodes[*source];
    let (path, kind) =
        contract(&consumer.resource_type, &r.attribute).ok_or(Failure::UnsupportedContract)?;
    proof::endpoint(endpoint)?;
    if endpoint.resource_type != kind {
        return Err(Failure::UnsupportedContract);
    }
    if *source == r.target {
        return Err(Failure::IncompleteProvenance);
    }
    let attributes = r
        .source_attributes_by_instance
        .get(source)
        .ok_or(Failure::IncompleteProvenance)?;
    if attributes.len() != 1 {
        return Err(Failure::OutputMismatch);
    }
    let attribute = attributes.first().ok_or(Failure::OutputMismatch)?.as_str();
    if attribute != "id"
        && !(attribute == "zone_id"
            && endpoint.mode == crate::model::EntityMode::Data
            && kind == "cloudflare_zone")
    {
        return Err(Failure::OutputMismatch);
    }
    let expected = evidence[r.target].id(&path.join("."))?;
    let value = evidence[*source];
    if value.id(attribute)? != expected
        || value.id("id")? != expected
        || (kind == "cloudflare_zone"
            && endpoint.mode == crate::model::EntityMode::Data
            && value.id("zone_id")? != expected)
    {
        return Err(Failure::IdentityMismatch);
    }
    let candidates = nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            current(node)
                && node.resource_type == kind
                && evidence[*index].id("id").ok() == Some(expected)
        })
        .count();
    (candidates == 1)
        .then_some(*source)
        .ok_or(Failure::AmbiguousEndpoint)
}

pub(super) fn annotate(
    nodes: &mut [TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[Evidence<'_>],
) {
    for (node, &values) in nodes.iter_mut().zip(evidence) {
        if node.provider.is_cloudflare() && scope_valid(node, values).is_err() {
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
        proof::record(&mut refs[index], source.map(|_| ()));
        if refs[index].attribute == "zone_id" {
            if let Ok(source) = source {
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
