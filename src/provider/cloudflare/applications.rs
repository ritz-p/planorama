use super::proof;
use crate::model::{AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_worker_version", "worker_id") => Some("Worker"),
        ("cloudflare_workers_script" | "cloudflare_worker_version", "bindings") => {
            Some("Storage bindings")
        }
        ("cloudflare_workers_deployment", "script_name") => Some("Deployed Worker"),
        ("cloudflare_workers_deployment", "versions") => Some("Configured versions"),
        ("cloudflare_workers_route", "script")
        | ("cloudflare_workers_custom_domain", "service") => Some("Configured Worker"),
        ("cloudflare_pages_domain", "project_name") => Some("Pages project"),
        ("cloudflare_pages_project", "deployment_configs") => Some("Storage bindings"),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Worker,
    WorkerName,
    Version,
    Pages,
    R2,
    Kv,
    D1,
}
struct Target<'a> {
    value: &'a str,
    kind: Kind,
    jurisdiction: Option<&'a str>,
}
impl<'a> Target<'a> {
    fn new(value: &'a str, kind: Kind) -> Self {
        Self {
            value,
            kind,
            jurisdiction: None,
        }
    }
}
fn jurisdiction<'a>(v: &'a Value, m: &Value) -> Option<&'a str> {
    let (v, m) = proof::at(v, m, "jurisdiction")?;
    if !matches!(m, Value::Null | Value::Bool(false)) {
        return None;
    }
    match v.as_str() {
        None if v.is_null() => Some("default"),
        Some("default" | "eu" | "fedramp" | "us" | "fedramp-high") => v.as_str(),
        _ => None,
    }
}
fn optional<'a>(v: &'a Value, m: &Value, path: &str) -> Option<Option<&'a Value>> {
    let (v, m) = proof::at(v, m, path)?;
    if v.is_null() && matches!(m, Value::Null | Value::Bool(false)) {
        Some(None)
    } else if !v.is_null() {
        Some(Some(v))
    } else {
        None
    }
}
fn bindings<'a>(v: &'a Value, m: &Value, version: bool) -> Option<Vec<Target<'a>>> {
    let mut targets = Vec::new();
    for (row, mask) in proof::rows(v, m)? {
        let target = match proof::text(row, mask, "type")? {
            "kv_namespace" => Target::new(proof::text(row, mask, "namespace_id")?, Kind::Kv),
            "r2_bucket" => {
                let jurisdiction = jurisdiction(row, mask)?;
                if !version && jurisdiction == "us" {
                    return None;
                }
                Target {
                    value: proof::text(row, mask, "bucket_name")?,
                    kind: Kind::R2,
                    jurisdiction: Some(jurisdiction),
                }
            }
            "d1" => {
                let id = proof::text(row, mask, "id")?;
                let (alias, alias_mask) = proof::at(row, mask, "database_id")?;
                if let Some(alias) = proof::text(row, mask, "database_id") {
                    if alias != id {
                        return None;
                    }
                } else if !alias.is_null() || (!version && !proof::unmasked(alias_mask)) {
                    return None;
                }
                Target::new(id, Kind::D1)
            }
            _ => continue,
        };
        targets.push(target);
    }
    Some(targets)
}
fn pages<'a>(v: &'a Value, m: &Value) -> Option<Vec<Target<'a>>> {
    let mut targets = Vec::new();
    for environment in ["preview", "production"] {
        if optional(v, m, environment)?.is_none() {
            continue;
        }
        let (env, mask) = proof::at(v, m, environment)?;
        for (field, identity, kind) in [
            ("d1_databases", "id", Kind::D1),
            ("kv_namespaces", "namespace_id", Kind::Kv),
            ("r2_buckets", "name", Kind::R2),
        ] {
            if optional(env, mask, field)?.is_none() {
                continue;
            }
            let (map, masks) = proof::at(env, mask, field)?;
            if !matches!(masks, Value::Null | Value::Bool(false) | Value::Object(_))
                || masks
                    .as_object()
                    .is_some_and(|m| m.keys().any(|k| map.get(k).is_none()))
            {
                return None;
            }
            for (key, row) in map.as_object()? {
                targets.push(Target {
                    value: proof::text(row, &masks[key], identity)?,
                    kind,
                    jurisdiction: if matches!(kind, Kind::R2) {
                        Some(jurisdiction(row, &masks[key])?)
                    } else {
                        None
                    },
                });
            }
        }
    }
    Some(targets)
}
fn targets<'a>(
    node: &TerraformEntity,
    path: &str,
    v: &'a Value,
    m: &Value,
) -> Option<Vec<Target<'a>>> {
    let (value, mask) = proof::at(v, m, path)?;
    match path {
        "bindings" => bindings(
            value,
            mask,
            node.resource_type == "cloudflare_worker_version",
        ),
        "deployment_configs" => pages(value, mask),
        "versions" => {
            let mut targets = Vec::new();
            let mut seen = BTreeSet::new();
            let mut total = 0.0;
            for (row, mask) in proof::rows(value, mask)? {
                let id = proof::text(row, mask, "version_id")?;
                let (percentage, mask) = proof::at(row, mask, "percentage")?;
                if !matches!(mask, Value::Null | Value::Bool(false)) {
                    return None;
                }
                let percentage = percentage.as_f64()?;
                if !(0.0..=100.0).contains(&percentage) || !seen.insert(id) {
                    return None;
                }
                total += percentage;
                targets.push(Target::new(id, Kind::Version));
            }
            ((total - 100.0).abs() < 0.000001).then_some(targets)
        }
        _ => Some(vec![Target::new(
            proof::text(v, m, path)?,
            match path {
                "worker_id" => Kind::Worker,
                "project_name" => Kind::Pages,
                "script_name" | "script" | "service" => Kind::WorkerName,
                _ => return None,
            },
        )]),
    }
}
fn identity(
    target: &Target<'_>,
    node: &TerraformEntity,
    v: &Value,
    m: &Value,
    output: &str,
) -> bool {
    let fields: &[&str] = match (target.kind, node.resource_type.as_str()) {
        (Kind::Worker, "cloudflare_worker") => &["id", "name"],
        (Kind::WorkerName, "cloudflare_worker") => &["name"],
        (Kind::WorkerName, "cloudflare_workers_script") => &["id", "script_name"],
        (Kind::Version, "cloudflare_worker_version") => &["id"],
        (Kind::Pages, "cloudflare_pages_project") => &["id", "name"],
        (Kind::R2, "cloudflare_r2_bucket") => &["id", "name"],
        (Kind::Kv, "cloudflare_workers_kv_namespace") => &["id"],
        (Kind::D1, "cloudflare_d1_database") => &["id", "uuid"],
        _ => return false,
    };
    fields.contains(&output)
        && proof::text(v, m, output) == Some(target.value)
        && (matches!(target.kind, Kind::Worker)
            || fields
                .iter()
                .all(|field| proof::text(v, m, field) == Some(target.value)))
        && (!matches!(target.kind, Kind::R2) || jurisdiction(v, m) == target.jurisdiction)
}
fn matching(target: &Target<'_>, node: &TerraformEntity, v: &Value, m: &Value) -> bool {
    ["id", "name", "script_name", "uuid"]
        .iter()
        .any(|output| identity(target, node, v, m, output))
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    if !proof::explicit(nodes, refs, r) {
        return false;
    }
    let node = &nodes[r.target];
    let (v, m) = evidence[r.target];
    let Some(targets) = targets(node, &r.attribute, v, m) else {
        return false;
    };
    if targets.is_empty() {
        return false;
    }
    let mut sources = BTreeSet::new();
    for target in targets {
        let mut candidates = nodes.iter().enumerate().filter(|(index, endpoint)| {
            let (v, m) = evidence[*index];
            proof::current(endpoint)
                && proof::same_account(node, endpoint)
                && matching(&target, endpoint, v, m)
        });
        let Some((index, endpoint)) = candidates.next() else {
            return false;
        };
        if candidates.next().is_some() || index == r.target || !r.sources.contains(&index) {
            return false;
        }
        let Some(outputs) = r.source_attributes_by_instance.get(&index) else {
            return false;
        };
        let (v, m) = evidence[index];
        if outputs.is_empty()
            || !outputs
                .iter()
                .all(|output| identity(&target, endpoint, v, m, output))
        {
            return false;
        }
        if matches!(target.kind, Kind::Version) && !version_owner(refs, r.target, index) {
            return false;
        }
        sources.insert(index);
    }
    sources == r.sources.iter().copied().collect()
}
fn version_owner(refs: &[AttributeReference], deployment: usize, version: usize) -> bool {
    let owner = |target, path: &str| {
        refs.iter()
            .find(|r| r.target == target && r.attribute == path && r.service_reference_verified)
            .and_then(|r| {
                if let [source] = r.sources.as_slice() {
                    Some(*source)
                } else {
                    None
                }
            })
    };
    owner(deployment, "script_name")
        .zip(owner(version, "worker_id"))
        .is_some_and(|(a, b)| a == b)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for versions in [false, true] {
        let results: Vec<_> = refs
            .iter()
            .map(|r| {
                let node = &nodes[r.target];
                (node.provider.is_cloudflare()
                    && node.mode == EntityMode::Managed
                    && label(&node.resource_type, &r.attribute).is_some()
                    && (r.attribute == "versions") == versions)
                    .then(|| verify(nodes, refs, r, evidence))
            })
            .collect();
        for (r, valid) in refs.iter_mut().zip(results) {
            if let Some(valid) = valid {
                r.service_reference_verified = valid;
            }
        }
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_applications.rs"]
mod tests;
