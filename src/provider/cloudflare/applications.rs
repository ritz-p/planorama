use super::{
    Evidence,
    evidence::{Result, collect},
    proof,
};
use crate::model::{AttributeReference, CloudflareFailure as Failure, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_dns_record", "content") => Some("Configured Pages CNAME"),
        ("cloudflare_worker_version", "worker_id") => Some("Worker"),
        ("cloudflare_workers_script" | "cloudflare_worker_version", "bindings") => {
            Some("Storage, service, queue and Hyperdrive bindings")
        }
        ("cloudflare_workers_deployment", "script_name") => Some("Deployed Worker"),
        ("cloudflare_workers_deployment", "versions") => Some("Configured versions"),
        ("cloudflare_workers_route", "script")
        | ("cloudflare_workers_custom_domain", "service") => Some("Configured Worker"),
        ("cloudflare_queue_consumer", "queue_id") => Some("Consumed Queue"),
        ("cloudflare_queue_consumer", "script_name") => Some("Consumer Worker"),
        ("cloudflare_queue_consumer", "dead_letter_queue") => Some("Configured dead letter Queue"),
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
    QueueId,
    QueueName,
    Hyperdrive,
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
fn jurisdiction(value: Evidence<'_>) -> Result<&str> {
    let field = value.at("jurisdiction")?;
    field.scalar()?;
    match field.value.as_str() {
        None if field.value.is_null() => Ok("default"),
        Some(value @ ("default" | "eu" | "fedramp" | "us" | "fedramp-high")) => Ok(value),
        _ => Err(Failure::UnsupportedContract),
    }
}
fn optional<'a>(value: Evidence<'a>, path: &str) -> Result<Option<Evidence<'a>>> {
    let field = value.at(path)?;
    if field.value.is_null() && matches!(field.mask, Value::Null | Value::Bool(false)) {
        Ok(None)
    } else if !field.value.is_null() {
        Ok(Some(field))
    } else {
        field.scalar()?;
        Err(Failure::InvalidValue)
    }
}
fn bindings(value: Evidence<'_>, version: bool) -> Result<Vec<Target<'_>>> {
    Ok(collect(value.rows()?.into_iter().map(|row| {
        let target = match row.text("type")? {
            "service" => {
                let environment = row.at("environment")?;
                environment.scalar()?;
                if !environment.value.is_null() {
                    return Err(Failure::UnsupportedContract);
                }
                let entrypoint = row.at("entrypoint")?;
                entrypoint.scalar()?;
                if !entrypoint.value.is_null() {
                    row.text("entrypoint")?;
                }
                Target::new(row.text("service")?, Kind::WorkerName)
            }
            "queue" => Target::new(row.text("queue_name")?, Kind::QueueName),
            "hyperdrive" => Target::new(row.text("id")?, Kind::Hyperdrive),
            "kv_namespace" => Target::new(row.text("namespace_id")?, Kind::Kv),
            "r2_bucket" => {
                let jurisdiction = jurisdiction(row)?;
                if !version && jurisdiction == "us" {
                    return Err(Failure::UnsupportedContract);
                }
                Target {
                    value: row.text("bucket_name")?,
                    kind: Kind::R2,
                    jurisdiction: Some(jurisdiction),
                }
            }
            "d1" => {
                let id = row.text("id")?;
                let alias = row.at("database_id")?;
                if let Ok(alias) = row.text("database_id") {
                    if alias != id {
                        return Err(Failure::IdentityMismatch);
                    }
                } else if !alias.value.is_null() || (!version && !proof::unmasked(alias.mask)) {
                    alias.scalar()?;
                    return Err(Failure::IdentityMismatch);
                }
                Target::new(id, Kind::D1)
            }
            _ => return Ok(None),
        };
        Ok(Some(target))
    }))?
    .into_iter()
    .flatten()
    .collect())
}
fn pages(value: Evidence<'_>) -> Result<Vec<Target<'_>>> {
    Ok(
        collect(["preview", "production"].into_iter().map(|environment| {
            let Some(env) = optional(value, environment)? else {
                return Ok(Vec::new());
            };
            Ok(collect(
                [
                    ("d1_databases", "id", Kind::D1),
                    ("kv_namespaces", "namespace_id", Kind::Kv),
                    ("r2_buckets", "name", Kind::R2),
                ]
                .into_iter()
                .map(|(field, identity, kind)| {
                    let Some(map) = optional(env, field)? else {
                        return Ok(Vec::new());
                    };
                    collect(map.map()?.into_iter().map(|(_, row)| {
                        Ok(Target {
                            value: row.text(identity)?,
                            kind,
                            jurisdiction: if matches!(kind, Kind::R2) {
                                Some(jurisdiction(row)?)
                            } else {
                                None
                            },
                        })
                    }))
                }),
            )?
            .into_iter()
            .flatten()
            .collect())
        }))?
        .into_iter()
        .flatten()
        .collect(),
    )
}
fn targets<'a>(node: &TerraformEntity, path: &str, v: Evidence<'a>) -> Result<Vec<Target<'a>>> {
    let value = v.at(path)?;
    if node.resource_type == "cloudflare_queue_consumer" {
        let kind = match (v.text("type")?, path) {
            ("worker" | "http_pull" | "notification", "queue_id") => Kind::QueueId,
            ("worker", "script_name") => Kind::WorkerName,
            ("worker", "dead_letter_queue") => Kind::QueueName,
            _ => return Err(Failure::UnsupportedContract),
        };
        return Ok(vec![Target::new(v.text(path)?, kind)]);
    }
    match path {
        "bindings" => bindings(value, node.resource_type == "cloudflare_worker_version"),
        "deployment_configs" => pages(value),
        "versions" => {
            let mut targets = Vec::new();
            let mut seen = BTreeSet::new();
            let mut total = 0.0;
            for (id, percentage) in collect(value.rows()?.into_iter().map(|row| {
                let id = row.text("version_id")?;
                let percentage = row.at("percentage")?;
                percentage.scalar()?;
                Ok((id, percentage.value.as_f64().ok_or(Failure::InvalidValue)?))
            }))? {
                if !(0.0..=100.0).contains(&percentage) || !seen.insert(id) {
                    return Err(Failure::IncompleteCollection);
                }
                total += percentage;
                targets.push(Target::new(id, Kind::Version));
            }
            ((total - 100.0).abs() < 0.000001)
                .then_some(targets)
                .ok_or(Failure::IncompleteCollection)
        }
        _ => Ok(vec![Target::new(
            v.text(path)?,
            match path {
                "worker_id" => Kind::Worker,
                "project_name" => Kind::Pages,
                "script_name" | "script" | "service" => Kind::WorkerName,
                _ => return Err(Failure::UnsupportedContract),
            },
        )]),
    }
}
fn fields(target: &Target<'_>, node: &TerraformEntity) -> Result<&'static [&'static str]> {
    Ok(match (target.kind, node.resource_type.as_str()) {
        (Kind::Worker, "cloudflare_worker") => &["id", "name"],
        (Kind::WorkerName, "cloudflare_worker") => &["name"],
        (Kind::WorkerName, "cloudflare_workers_script") => &["id", "script_name"],
        (Kind::Version, "cloudflare_worker_version") => &["id"],
        (Kind::Pages, "cloudflare_pages_project") => &["id", "name"],
        (Kind::R2, "cloudflare_r2_bucket") => &["id", "name"],
        (Kind::Kv, "cloudflare_workers_kv_namespace") => &["id"],
        (Kind::D1, "cloudflare_d1_database") => &["id", "uuid"],
        (Kind::QueueId, "cloudflare_queue") => &["id", "queue_id"],
        (Kind::QueueName, "cloudflare_queue") => &["queue_name"],
        (Kind::Hyperdrive, "cloudflare_hyperdrive_config") => &["id"],
        _ => return Err(Failure::UnsupportedContract),
    })
}
fn identity(
    target: &Target<'_>,
    node: &TerraformEntity,
    value: Evidence<'_>,
    output: &str,
) -> Result<()> {
    let fields = fields(target, node)?;
    if !fields.contains(&output) {
        return Err(Failure::OutputMismatch);
    }
    if value.text(output)? != target.value {
        return Err(Failure::IdentityMismatch);
    }
    if !matches!(target.kind, Kind::Worker)
        && collect(fields.iter().map(|field| value.text(field)))?
            .iter()
            .any(|id| *id != target.value)
    {
        return Err(Failure::IdentityMismatch);
    }
    if matches!(target.kind, Kind::R2) && Some(jurisdiction(value)?) != target.jurisdiction {
        return Err(Failure::IdentityMismatch);
    }
    Ok(())
}
fn matching(target: &Target<'_>, node: &TerraformEntity, value: Evidence<'_>) -> bool {
    [
        "id",
        "name",
        "script_name",
        "uuid",
        "queue_id",
        "queue_name",
    ]
    .iter()
    .any(|output| identity(target, node, value, output).is_ok())
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<()> {
    proof::explicit(nodes, refs, r)?;
    let node = &nodes[r.target];
    let targets = targets(node, &r.attribute, evidence[r.target])?;
    if targets.is_empty() {
        return Err(Failure::UnsupportedContract);
    }
    let sources: BTreeSet<_> = collect(targets.into_iter().map(|target| {
        // Rejected candidates are expected; only explicit sources explain a failed match.
        let mut candidates = nodes.iter().enumerate().filter(|(index, endpoint)| {
            proof::current(endpoint)
                && proof::account(node, endpoint, evidence[r.target], evidence[*index]).is_ok()
                && matching(&target, endpoint, evidence[*index])
        });
        let Some((index, endpoint)) = candidates.next() else {
            let errors =
                r.sources
                    .iter()
                    .filter(|&&index| fields(&target, &nodes[index]).is_ok())
                    .map(|&index| -> Result<()> {
                        proof::endpoint(&nodes[index])?;
                        proof::account(node, &nodes[index], evidence[r.target], evidence[index])?;
                        let outputs = r
                            .source_attributes_by_instance
                            .get(&index)
                            .filter(|outputs| !outputs.is_empty())
                            .ok_or(Failure::IncompleteProvenance)?;
                        collect(outputs.iter().map(|output| {
                            identity(&target, &nodes[index], evidence[index], output)
                        }))
                        .map(|_| ())
                    });
            return Err(errors
                .filter_map(Result::err)
                .min()
                .unwrap_or(Failure::UnsupportedContract));
        };
        if candidates.next().is_some() {
            return Err(Failure::AmbiguousEndpoint);
        }
        if index == r.target || !r.sources.contains(&index) {
            return Err(Failure::IncompleteProvenance);
        }
        if r.attribute == "dead_letter_queue" {
            let owner = refs
                .iter()
                .find(|owner| {
                    owner.target == r.target
                        && owner.attribute == "queue_id"
                        && owner.service_reference_verified
                })
                .ok_or(Failure::OwnerMismatch)?;
            if owner.sources.contains(&index) {
                return Err(Failure::OwnerMismatch);
            }
        }
        let outputs = r
            .source_attributes_by_instance
            .get(&index)
            .filter(|outputs| !outputs.is_empty())
            .ok_or(Failure::IncompleteProvenance)?;
        collect(
            outputs
                .iter()
                .map(|output| identity(&target, endpoint, evidence[index], output)),
        )?;
        if matches!(target.kind, Kind::Version) && !version_owner(refs, r.target, index) {
            return Err(Failure::OwnerMismatch);
        }
        Ok(index)
    }))?
    .into_iter()
    .collect();
    (sources == r.sources.iter().copied().collect())
        .then_some(())
        .ok_or(Failure::IncompleteProvenance)
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

pub(super) fn present(plan: &crate::model::TerraformPlan, graph: &mut crate::model::Graph) {
    let roles: std::collections::BTreeMap<_, _> = plan
        .nodes
        .iter()
        .filter(|node| node.provider.is_cloudflare())
        .filter_map(|node| {
            let label = match node.resource_type.as_str() {
                "cloudflare_worker" | "cloudflare_workers_script" => "Service bindings",
                "cloudflare_queue" => "Queue producer bindings",
                "cloudflare_hyperdrive_config" => "Hyperdrive bindings",
                "cloudflare_r2_bucket"
                | "cloudflare_workers_kv_namespace"
                | "cloudflare_d1_database" => "Storage bindings",
                _ => return None,
            };
            Some((crate::model::TerraformEntityId::from(node), label))
        })
        .collect();
    for node in &mut graph.nodes {
        if !node.provider.is_cloudflare()
            || !matches!(
                node.resource_type.as_str(),
                "cloudflare_worker_version" | "cloudflare_workers_script"
            )
        {
            continue;
        }
        let mut split = Vec::new();
        for reference in &mut node.references {
            if reference.dependency || reference.attribute != "bindings" || !reference.complete {
                continue;
            }
            let mut groups = std::collections::BTreeMap::<_, Vec<_>>::new();
            for target in reference.targets.drain(..) {
                groups
                    .entry(roles.get(&target.id).copied().unwrap_or(reference.label))
                    .or_default()
                    .push(target);
            }
            let mut groups = groups.into_iter();
            if let Some((label, targets)) = groups.next() {
                reference.label = label;
                reference.targets = targets;
                for (label, targets) in groups {
                    let mut role = reference.clone();
                    role.label = label;
                    role.targets = targets;
                    split.push(role);
                }
            }
        }
        node.references.extend(split);
    }
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[Evidence<'_>],
) {
    for versions in [false, true] {
        let results: Vec<_> = refs
            .iter()
            .map(|r| {
                let node = &nodes[r.target];
                (node.provider.is_cloudflare()
                    && label(&node.resource_type, &r.attribute).is_some()
                    && matches!(r.attribute.as_str(), "versions" | "dead_letter_queue") == versions)
                    .then(|| {
                        if node.mode == EntityMode::Managed {
                            if node.resource_type == "cloudflare_dns_record" {
                                dns_pages(nodes, refs, r, evidence)
                            } else {
                                verify(nodes, refs, r, evidence)
                            }
                        } else {
                            Err(Failure::UnsupportedContract)
                        }
                    })
            })
            .collect();
        for (r, valid) in refs.iter_mut().zip(results) {
            if let Some(valid) = valid {
                proof::record(r, valid);
            }
        }
    }
    reject_dlq_cycles(refs);
}

fn reject_dlq_cycles(refs: &mut [AttributeReference]) {
    let edges: Vec<_> = refs
        .iter()
        .enumerate()
        .filter_map(|(index, r)| {
            if r.attribute != "dead_letter_queue" || !r.service_reference_verified {
                return None;
            }
            let owner = refs.iter().find(|owner| {
                owner.target == r.target
                    && owner.attribute == "queue_id"
                    && owner.service_reference_verified
            })?;
            Some((index, *owner.sources.first()?, *r.sources.first()?))
        })
        .collect();
    for &(index, owner, target) in &edges {
        let mut pending = vec![target];
        let mut visited = BTreeSet::new();
        while let Some(node) = pending.pop() {
            if node == owner {
                proof::record(&mut refs[index], Err(Failure::OwnerMismatch));
                break;
            }
            if visited.insert(node) {
                pending.extend(
                    edges
                        .iter()
                        .filter(|(_, from, _)| *from == node)
                        .map(|(_, _, to)| *to),
                );
            }
        }
    }
}

fn dns_pages(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<()> {
    let value = evidence[r.target];
    if value.text("type")? != "CNAME" {
        return Err(Failure::UnsupportedContract);
    }
    let proxied = value.at("proxied")?;
    proxied.scalar()?;
    if !proxied.value.is_boolean() {
        return Err(Failure::InvalidValue);
    }
    if !refs.iter().any(|owner| {
        owner.target == r.target && owner.attribute == "zone_id" && owner.service_reference_verified
    }) {
        return Err(Failure::OwnerMismatch);
    }
    proof::verify(
        nodes,
        refs,
        r,
        evidence,
        &[proof::Target {
            value: value.text("content")?,
            kind: "cloudflare_pages_project",
            output: "subdomain",
        }],
        |a, b| proof::account(&nodes[a], &nodes[b], evidence[a], evidence[b]),
    )
}
#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_applications.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_queues.rs"]
mod queue_tests;
