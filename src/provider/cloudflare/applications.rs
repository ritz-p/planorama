mod deployment;
mod dns;
mod pages;
mod presentation;
mod queues;
mod target;
mod workers;
use super::{
    Evidence,
    evidence::{Result, collect},
    proof,
};
use crate::model::{AttributeReference, CloudflareFailure as Failure, EntityMode, TerraformEntity};

use std::collections::BTreeSet;
use target::{Kind, Target, fields, identity, matching};
pub(super) fn present(plan: &crate::model::TerraformPlan, graph: &mut crate::model::Graph) {
    presentation::present(plan, graph);
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if (kind, path) == ("cloudflare_ai_gateway_dynamic_routing", "gateway_id") {
        return Some("Configured AI Gateway");
    }
    if kind == "cloudflare_ai_search_instance" {
        return match path {
            "namespace" => Some("Instance namespace"),
            "ai_gateway_id" => Some("Search AI Gateway"),
            "token_id" => Some("Search token"),
            "source" => Some("Search source bucket"),
            _ => None,
        };
    }
    workers::label(kind, path)
        .or_else(|| pages::label(kind, path))
        .or_else(|| queues::label(kind, path))
        .or_else(|| deployment::label(kind, path))
        .or_else(|| dns::label(kind, path))
}
fn targets<'a>(node: &TerraformEntity, path: &str, v: Evidence<'a>) -> Result<Vec<Target<'a>>> {
    let value = v.at(path)?;
    if node.resource_type == "cloudflare_queue_consumer" {
        return queues::targets(v, path);
    }
    if node.resource_type == "cloudflare_r2_bucket_event_notification" {
        let target = match path {
            "bucket_name" => Target {
                value: v.text(path)?,
                kind: Kind::R2,
                jurisdiction: Some(target::jurisdiction(v)?),
            },
            "queue_id" => Target::new(v.text(path)?, Kind::QueueId),
            _ => return Err(Failure::UnsupportedContract),
        };
        return Ok(vec![target]);
    }
    match path {
        "source" if node.resource_type == "cloudflare_ai_search_instance" => {
            if v.text("type")? != "r2" {
                return Err(Failure::UnsupportedContract);
            }
            let params = v.at("source_params")?;
            let jurisdiction = if params.value.is_null() {
                params.require_null()?;
                "default"
            } else {
                let field = params.at("r2_jurisdiction")?;
                field.scalar()?;
                if field.value.is_null() {
                    "default"
                } else {
                    params.text("r2_jurisdiction")?
                }
            };
            Ok(vec![Target {
                value: v.text(path)?,
                kind: Kind::R2,
                jurisdiction: Some(jurisdiction),
            }])
        }
        "bindings" => workers::bindings(value, node.resource_type == "cloudflare_worker_version"),
        "deployment_configs" => pages::targets(value),
        "versions" => deployment::targets(value),
        _ => Ok(vec![Target::new(
            v.text(path)?,
            match path {
                "worker_id" => Kind::Worker,
                "project_name" => Kind::Pages,
                "gateway_id" | "ai_gateway_id" => Kind::AiGateway,
                "token_id" => Kind::SearchToken,
                "namespace" if node.configurable_search_namespace => Kind::SearchNamespace,
                "script_name" | "script" | "service" => Kind::WorkerName,
                _ => return Err(Failure::UnsupportedContract),
            },
        )]),
    }
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<BTreeSet<usize>> {
    proof::explicit(nodes, refs, r)?;
    let node = &nodes[r.target];
    let targets = targets(node, &r.attribute, evidence[r.target])?;
    if targets.is_empty() {
        return Err(Failure::UnsupportedContract);
    }
    let proven = collect(targets.into_iter().map(|target| {
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
        queues::owner(refs, r, index)?;
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
        if matches!(target.kind, Kind::Version) && !deployment::version_owner(refs, r.target, index)
        {
            return Err(Failure::OwnerMismatch);
        }
        Ok((index, matches!(target.kind, Kind::DurableObjectScript)))
    }))?;
    let mut sources = BTreeSet::new();
    let mut scripts = BTreeSet::new();
    let mut other = BTreeSet::new();
    for (index, durable) in proven {
        sources.insert(index);
        if durable {
            scripts.insert(index);
        } else {
            other.insert(index);
        }
    }
    (sources == r.sources.iter().copied().collect() && scripts.is_disjoint(&other))
        .then_some(scripts)
        .ok_or(Failure::IncompleteProvenance)
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
                                dns::verify(nodes, refs, r, evidence).map(|()| BTreeSet::new())
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
                let valid = valid.map(|scripts| r.durable_object_scripts = scripts);
                proof::record(r, valid);
                if r.service_reference_verified && r.attribute == "versions" {
                    match deployment::percentages(r, evidence) {
                        Ok(values) => r.cloudflare_percentages = values,
                        Err(failure) => proof::record(r, Err(failure)),
                    }
                }
            }
        }
    }
    queues::reject_dlq_cycles(refs);
}
#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_applications.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_queues.rs"]
mod queue_tests;

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_dns_public.rs"]
mod dns_public_tests;
