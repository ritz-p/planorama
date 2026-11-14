use super::proof::{self, Target};
use crate::model::{AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_zero_trust_access_application", "allowed_idps") => {
            Some("Configured identity providers")
        }
        ("cloudflare_zero_trust_access_application", "policies") => {
            Some("Configured access policies")
        }
        (
            "cloudflare_zero_trust_tunnel_cloudflared_config"
            | "cloudflare_zero_trust_tunnel_cloudflared_route",
            "tunnel_id",
        ) => Some("Configured tunnel"),
        ("cloudflare_zero_trust_tunnel_cloudflared_route", "virtual_network_id") => {
            Some("Tunnel virtual network")
        }
        _ => None,
    }
}
fn targets<'a>(path: &str, value: &'a Value, mask: &Value) -> Option<Vec<Target<'a>>> {
    let (v, m) = proof::at(value, mask, path)?;
    let target = |value, kind| Target {
        value,
        kind,
        output: "id",
    };
    match path {
        "allowed_idps" => {
            if !proof::unmasked(m) {
                return None;
            }
            Some(
                proof::list(v, m)?
                    .into_iter()
                    .map(|id| target(id, "cloudflare_zero_trust_access_identity_provider"))
                    .collect(),
            )
        }
        "policies" => {
            let mut targets = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for (row, mask) in proof::rows(v, m)? {
                let id = proof::text(row, mask, "id")?;
                if !seen.insert(id) {
                    return None;
                }
                targets.push(target(id, "cloudflare_zero_trust_access_policy"));
            }
            Some(targets)
        }
        "tunnel_id" => Some(vec![target(
            proof::text(value, mask, path)?,
            "cloudflare_zero_trust_tunnel_cloudflared",
        )]),
        "virtual_network_id" => Some(vec![target(
            proof::text(value, mask, path)?,
            "cloudflare_zero_trust_tunnel_cloudflared_virtual_network",
        )]),
        _ => None,
    }
}
fn compatible(a: &TerraformEntity, b: &TerraformEntity) -> bool {
    if b.resource_type == "cloudflare_zero_trust_access_identity_provider" {
        if let Some(zone) = b.cloudflare_scope.as_ref().and_then(|s| s.zone.as_ref()) {
            return a.cloudflare_scope.as_ref().and_then(|s| s.zone.as_ref()) == Some(zone);
        }
    }
    proof::same_account(a, b)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let node = &nodes[r.target];
            if !node.provider.is_cloudflare()
                || node.mode != EntityMode::Managed
                || label(&node.resource_type, &r.attribute).is_none()
            {
                return None;
            }
            let (v, m) = evidence[r.target];
            Some(targets(&r.attribute, v, m).is_some_and(|targets| {
                proof::verify(nodes, refs, r, evidence, &targets, compatible)
            }))
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_zero_trust.rs"]
mod tests;
