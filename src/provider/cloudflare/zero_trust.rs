use super::{
    Evidence,
    evidence::{Result, collect},
    proof::{self, Target},
};
use crate::model::{AttributeReference, CloudflareFailure as Failure, EntityMode, TerraformEntity};

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_zero_trust_access_policy", "include") => Some("Included access groups"),
        ("cloudflare_zero_trust_access_policy", "require") => Some("Required access groups"),
        ("cloudflare_zero_trust_access_policy", "exclude") => Some("Excluded access groups"),
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

fn targets<'a>(path: &str, value: Evidence<'a>) -> Result<Vec<Target<'a>>> {
    let field = value.at(path)?;
    let target = |value, kind| Target {
        value,
        kind,
        output: "id",
    };
    match path {
        "include" | "require" | "exclude" => {
            field.clear()?;
            let ids = collect(field.rows()?.into_iter().map(|row| {
                let group = row.at("group")?;
                if group.value.is_null() {
                    Ok(None)
                } else {
                    group.text("id").map(Some)
                }
            }))?;
            let ids: Vec<_> = ids.into_iter().flatten().collect();
            proof::unique(&ids)?;
            Ok(ids
                .into_iter()
                .map(|id| target(id, "cloudflare_zero_trust_access_group"))
                .collect())
        }
        "allowed_idps" => {
            field.clear()?;
            Ok(field
                .list()?
                .into_iter()
                .map(|id| target(id, "cloudflare_zero_trust_access_identity_provider"))
                .collect())
        }
        "policies" => {
            let ids = collect(field.rows()?.into_iter().map(|row| row.text("id")))?;
            proof::unique(&ids)?;
            Ok(ids
                .into_iter()
                .map(|id| target(id, "cloudflare_zero_trust_access_policy"))
                .collect())
        }
        "tunnel_id" => Ok(vec![target(
            value.text(path)?,
            "cloudflare_zero_trust_tunnel_cloudflared",
        )]),
        "virtual_network_id" => Ok(vec![target(
            value.text(path)?,
            "cloudflare_zero_trust_tunnel_cloudflared_virtual_network",
        )]),
        _ => Err(Failure::UnsupportedContract),
    }
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<()> {
    proof::explicit(nodes, refs, r)?;
    proof::verify(
        nodes,
        refs,
        r,
        evidence,
        &targets(&r.attribute, evidence[r.target])?,
        |a, b| {
            if nodes[b].resource_type == "cloudflare_zero_trust_access_group" {
                let zone = evidence[b].at("zone_id")?;
                zone.scalar()?;
                if !zone.value.is_null() {
                    return Err(Failure::ScopeMismatch);
                }
                if nodes[b].mode == EntityMode::Data
                    && evidence[b].text("group_id")? != evidence[b].text("id")?
                {
                    return Err(Failure::IdentityMismatch);
                }
            }
            if nodes[b].resource_type == "cloudflare_zero_trust_access_identity_provider" {
                if let Some(zone) = nodes[b]
                    .cloudflare_scope
                    .as_ref()
                    .and_then(|s| s.zone.as_ref())
                {
                    let consumer = nodes[a]
                        .cloudflare_scope
                        .as_ref()
                        .and_then(|s| s.zone.as_ref())
                        .ok_or_else(|| proof::scope_failure(&nodes[a], evidence[a]))?;
                    return (consumer == zone)
                        .then_some(())
                        .ok_or(Failure::ScopeMismatch);
                }
            }
            proof::account(&nodes[a], &nodes[b], evidence[a], evidence[b])
        },
    )
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[Evidence<'_>],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let node = &nodes[r.target];
            (node.provider.is_cloudflare() && label(&node.resource_type, &r.attribute).is_some())
                .then(|| {
                    if node.mode != EntityMode::Managed {
                        Err(Failure::UnsupportedContract)
                    } else {
                        verify(nodes, refs, r, evidence)
                    }
                })
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(result) = result {
            proof::record(r, result);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_zero_trust.rs"]
mod tests;
