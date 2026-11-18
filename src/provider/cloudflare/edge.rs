use super::{
    Evidence,
    evidence::{Result, collect},
    proof::{self, Target},
};
use crate::model::{AttributeReference, CloudflareFailure as Failure, TerraformEntity};

pub(super) fn pool_origins(
    value: Evidence<'_>,
) -> Result<Vec<crate::model::architecture::PoolOrigin>> {
    collect(value.at("origins")?.rows()?.into_iter().map(|row| {
        let enabled = row.at("enabled")?;
        let weight = row.at("weight")?;
        enabled.scalar()?;
        weight.scalar()?;
        if !(enabled.value.is_null() || enabled.value.is_boolean())
            || !(weight.value.is_null() || weight.value.is_number())
        {
            return Err(Failure::InvalidValue);
        }
        Ok(crate::model::architecture::PoolOrigin {
            enabled: enabled.value.as_bool(),
            weight: weight.value.as_number().map(ToString::to_string),
        })
    }))
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_load_balancer", "default_pools") => Some("Default pools"),
        ("cloudflare_load_balancer", "fallback_pool") => Some("Fallback pool"),
        ("cloudflare_load_balancer", "country_pools" | "region_pools" | "pop_pools") => {
            Some("Location pools")
        }
        ("cloudflare_load_balancer_pool", "monitor") => Some("Health monitor"),
        ("cloudflare_load_balancer_pool", "monitor_group") => Some("Health monitor group"),
        ("cloudflare_load_balancer_monitor_group", "members") => Some("Configured health monitors"),
        ("cloudflare_ruleset", "rules") => Some("Configured rule targets"),
        _ => None,
    }
}

fn targets<'a>(node: &TerraformEntity, path: &str, value: Evidence<'a>) -> Result<Vec<Target<'a>>> {
    let field = value.at(path)?;
    let target = |value, kind, output| Target {
        value,
        kind,
        output,
    };
    let pool = |id| target(id, "cloudflare_load_balancer_pool", "id");
    match (node.resource_type.as_str(), path) {
        ("cloudflare_load_balancer", "default_pools") => {
            Ok(field.list()?.into_iter().map(pool).collect())
        }
        ("cloudflare_load_balancer", "country_pools" | "region_pools" | "pop_pools") => Ok(
            collect(field.map()?.into_iter().map(|(_, value)| value.list()))?
                .into_iter()
                .flatten()
                .map(pool)
                .collect(),
        ),
        ("cloudflare_load_balancer", "fallback_pool") => Ok(vec![pool(value.text(path)?)]),
        ("cloudflare_load_balancer_pool", "monitor") => Ok(vec![target(
            value.text(path)?,
            "cloudflare_load_balancer_monitor",
            "id",
        )]),
        ("cloudflare_load_balancer_pool", "monitor_group") => Ok(vec![target(
            value.text(path)?,
            "cloudflare_load_balancer_monitor_group",
            "id",
        )]),
        ("cloudflare_load_balancer_monitor_group", "members") => {
            field.clear()?;
            let ids = collect(field.rows()?.into_iter().map(|row| row.text("monitor_id")))?;
            proof::unique(&ids)?;
            Ok(ids
                .into_iter()
                .map(|id| target(id, "cloudflare_load_balancer_monitor", "id"))
                .collect())
        }
        ("cloudflare_ruleset", "rules") => Ok(collect(field.rows()?.into_iter().map(|row| {
            Ok(match row.text("action")? {
                "execute" => Some(target(
                    row.text("action_parameters.id")?,
                    "cloudflare_ruleset",
                    "id",
                )),
                "redirect" => {
                    let list = row.at("action_parameters.from_list")?;
                    if !list.value.is_null() || !proof::unmasked(list.mask) {
                        Some(target(
                            row.text("action_parameters.from_list.name")?,
                            "cloudflare_list",
                            "name",
                        ))
                    } else {
                        None
                    }
                }
                _ => None,
            })
        }))?
        .into_iter()
        .flatten()
        .collect()),
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
    let targets = targets(&nodes[r.target], &r.attribute, evidence[r.target])?;
    proof::verify(nodes, refs, r, evidence, &targets, |a, b| {
        proof::account(&nodes[a], &nodes[b], evidence[a], evidence[b])?;
        if nodes[b].resource_type == "cloudflare_ruleset"
            && nodes[a]
                .cloudflare_scope
                .as_ref()
                .and_then(|s| s.zone.as_ref())
                != nodes[b]
                    .cloudflare_scope
                    .as_ref()
                    .and_then(|s| s.zone.as_ref())
        {
            return Err(Failure::ScopeMismatch);
        }
        Ok(())
    })?;
    collect(
        targets
            .iter()
            .filter(|target| target.kind == "cloudflare_list")
            .map(|target| {
                let results: Vec<_> = r
                    .sources
                    .iter()
                    .filter(|&&index| {
                        nodes[index].resource_type == "cloudflare_list"
                            && evidence[index].text("name").ok() == Some(target.value)
                    })
                    .map(|&index| {
                        (evidence[index].text("kind")? == "redirect")
                            .then_some(())
                            .ok_or(Failure::UnsupportedContract)
                    })
                    .collect();
                if results.iter().any(Result::is_ok) {
                    Ok(())
                } else {
                    Err(results
                        .into_iter()
                        .filter_map(Result::err)
                        .min()
                        .unwrap_or(Failure::UnsupportedContract))
                }
            }),
    )
    .map(|_| ())
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
                    if node.mode != crate::model::EntityMode::Managed {
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
#[path = "../../../tests/unit/cloudflare_edge.rs"]
mod tests;
