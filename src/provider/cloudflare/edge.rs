use super::proof::{self, Target};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

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

fn targets<'a>(
    node: &TerraformEntity,
    path: &str,
    value: &'a Value,
    mask: &Value,
) -> Option<Vec<Target<'a>>> {
    let (v, m) = proof::at(value, mask, path)?;
    let mut result = Vec::new();
    let target = |value, kind, output| Target {
        value,
        kind,
        output,
    };
    match (node.resource_type.as_str(), path) {
        ("cloudflare_load_balancer", "default_pools") => {
            for id in proof::list(v, m)? {
                result.push(target(id, "cloudflare_load_balancer_pool", "id"));
            }
        }
        ("cloudflare_load_balancer", "country_pools" | "region_pools" | "pop_pools") => {
            if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
                return None;
            }
            if m.as_object()
                .is_some_and(|fields| fields.keys().any(|key| v.get(key).is_none()))
            {
                return None;
            }
            for (key, values) in v.as_object()? {
                for id in proof::list(values, &m[key])? {
                    result.push(target(id, "cloudflare_load_balancer_pool", "id"));
                }
            }
        }
        ("cloudflare_load_balancer", "fallback_pool") => result.push(target(
            proof::text(value, mask, path)?,
            "cloudflare_load_balancer_pool",
            "id",
        )),
        ("cloudflare_load_balancer_pool", "monitor") => result.push(target(
            proof::text(value, mask, path)?,
            "cloudflare_load_balancer_monitor",
            "id",
        )),
        ("cloudflare_load_balancer_pool", "monitor_group") => result.push(target(
            proof::text(value, mask, path)?,
            "cloudflare_load_balancer_monitor_group",
            "id",
        )),
        ("cloudflare_load_balancer_monitor_group", "members") => {
            if !proof::unmasked(m) {
                return None;
            }
            let mut seen = std::collections::BTreeSet::new();
            for (row, mask) in proof::rows(v, m)? {
                let id = proof::text(row, mask, "monitor_id")?;
                if !seen.insert(id) {
                    return None;
                }
                result.push(target(id, "cloudflare_load_balancer_monitor", "id"));
            }
        }
        ("cloudflare_ruleset", "rules") => {
            for (row, mask) in proof::rows(v, m)? {
                match proof::text(row, mask, "action")? {
                    "execute" => result.push(target(
                        proof::text(row, mask, "action_parameters.id")?,
                        "cloudflare_ruleset",
                        "id",
                    )),
                    "redirect" => {
                        let (list, list_mask) =
                            proof::at(row, mask, "action_parameters.from_list")?;
                        if !list.is_null() || !proof::unmasked(list_mask) {
                            result.push(target(
                                proof::text(row, mask, "action_parameters.from_list.name")?,
                                "cloudflare_list",
                                "name",
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => return None,
    }
    Some(result)
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
                || node.mode != crate::model::EntityMode::Managed
                || label(&node.resource_type, &r.attribute).is_none()
            {
                return None;
            }
            let (value, mask) = evidence[r.target];
            let valid = targets(node, &r.attribute, value, mask).is_some_and(|targets| {
                proof::verify(nodes, refs, r, evidence, &targets, |a, b| {
                    proof::same_account(a, b)
                        && (b.resource_type != "cloudflare_ruleset"
                            || a.cloudflare_scope.as_ref().and_then(|s| s.zone.as_ref())
                                == b.cloudflare_scope.as_ref().and_then(|s| s.zone.as_ref()))
                }) && targets.iter().all(|target| {
                    if target.kind != "cloudflare_list" {
                        return true;
                    }
                    r.sources.iter().any(|&index| {
                        let (v, m) = evidence[index];
                        nodes[index].resource_type == "cloudflare_list"
                            && proof::text(v, m, "name") == Some(target.value)
                            && proof::text(v, m, "kind") == Some("redirect")
                    })
                })
            });
            Some(valid)
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_edge.rs"]
mod tests;
