use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/batch.rs"]
mod tests;

const SUBNETS: &str = "compute_resources.subnets";
const GROUPS: &str = "compute_resources.security_group_ids";
const ENVIRONMENTS: &str = "compute_environment_order.compute_environment";

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        ("aws_batch_compute_environment", SUBNETS) => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_batch_compute_environment", GROUPS) => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_batch_job_queue", ENVIRONMENTS) => Some((
            "aws_batch_compute_environment",
            "arn",
            true,
            "Compute environments",
        )),
        ("aws_batch_job_queue", "scheduling_policy_arn") => Some((
            "aws_batch_scheduling_policy",
            "arn",
            false,
            "Scheduling policy",
        )),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.3)
}

fn environments<'a>(value: &'a Value, unknown: &Value) -> Option<BTreeSet<&'a str>> {
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let entries = value["compute_environment_order"].as_array()?;
    if entries.is_empty() {
        return None;
    }
    let masks = match &unknown["compute_environment_order"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(masks) if masks.len() == entries.len() => Some(masks),
        _ => return None,
    };
    let mut result = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for (i, entry) in entries.iter().enumerate() {
        let mask = masks.map_or(&Value::Null, |m| &m[i]);
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
            || !matches!(&mask["order"], Value::Null | Value::Bool(false))
            || !matches!(
                &mask["compute_environment"],
                Value::Null | Value::Bool(false)
            )
            || !orders.insert(entry["order"].as_u64()?)
            || !result.insert(
                entry["compute_environment"]
                    .as_str()
                    .filter(|v| !v.is_empty())?,
            )
        {
            return None;
        }
    }
    Some(result)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !super::service_evidence::eligible(nodes, r)
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(field)
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        if node.resource_type == "aws_batch_compute_environment"
            && (super::service_evidence::known(evidence, r.target, "type") != Some("MANAGED")
                || !super::streaming::values(value, unknown, "compute_resources.type", false)
                    .is_some_and(|types| {
                        types.len() == 1
                            && types
                                .iter()
                                .all(|t| matches!(*t, "EC2" | "SPOT" | "FARGATE" | "FARGATE_SPOT"))
                    }))
        {
            continue;
        }
        let actual = if r.attribute == ENVIRONMENTS {
            environments(value, unknown)
        } else {
            super::streaming::values(value, unknown, &r.attribute, collection)
        };
        let Some(actual) = actual else { continue };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == kind)
                    .then(|| super::service_evidence::known(evidence, source, field))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_batch_compute_environment" {
            continue;
        }
        let proof = |attribute: &str| {
            let mut refs = raw
                .attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == attribute);
            let r = refs.next()?;
            (refs.next().is_none() && super::service_references::verified(raw, r)).then_some(r)
        };
        let Some((subnets, groups)) = proof(SUBNETS).zip(proof(GROUPS)) else {
            continue;
        };
        let members = subnets.sources.iter().chain(&groups.sources);
        let Some(parents) = members
            .clone()
            .map(|&i| super::streaming::vpc(raw, i))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().expect("one VPC");
        let mut evidence = Vec::new();
        for &member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
