use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/msk.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/mq.rs"]
mod mq_tests;

pub(super) fn network(resource: &str) -> Option<(&'static str, &'static str)> {
    match resource {
        "aws_msk_cluster" => Some((
            "broker_node_group_info.client_subnets",
            "broker_node_group_info.security_groups",
        )),
        "aws_msk_serverless_cluster" => {
            Some(("vpc_config.subnet_ids", "vpc_config.security_group_ids"))
        }
        "aws_msk_vpc_connection" => Some(("client_subnets", "security_groups")),
        "aws_mq_broker" => Some(("subnet_ids", "security_groups")),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    if let Some((subnets, groups)) = network(resource) {
        if attribute == subnets {
            return Some("Configured subnets");
        }
        if attribute == groups {
            return Some("Configured security groups");
        }
    }
    match (resource, attribute) {
        ("aws_msk_cluster", "configuration_info.arn") => Some("MSK configuration"),
        ("aws_mq_broker", "configuration.id") => Some("MQ configuration"),
        (
            "aws_msk_scram_secret_association" | "aws_msk_single_scram_secret_association",
            "cluster_arn",
        ) => Some("SCRAM cluster"),
        ("aws_msk_scram_secret_association", "secret_arn_list")
        | ("aws_msk_single_scram_secret_association", "secret_arn") => Some("SCRAM secrets"),
        ("aws_msk_vpc_connection", "target_cluster_arn") => Some("Client connection cluster"),
        ("aws_msk_vpc_connection", "vpc_id") => Some("Client VPC"),
        _ => None,
    }
}

fn rule(resource: &str, attribute: &str) -> Option<(&'static str, &'static str, bool)> {
    if let Some((subnets, groups)) = network(resource) {
        if attribute == subnets {
            return Some(("aws_subnet", "id", true));
        }
        if attribute == groups {
            return Some(("aws_security_group", "id", true));
        }
    }
    match (resource, attribute) {
        ("aws_msk_cluster", "configuration_info.arn") => {
            Some(("aws_msk_configuration", "arn", false))
        }
        ("aws_mq_broker", "configuration.id") => Some(("aws_mq_configuration", "id", false)),
        (
            "aws_msk_scram_secret_association" | "aws_msk_single_scram_secret_association",
            "cluster_arn",
        )
        | ("aws_msk_vpc_connection", "target_cluster_arn") => {
            Some(("aws_msk_cluster", "arn", false))
        }
        ("aws_msk_scram_secret_association", "secret_arn_list") => {
            Some(("aws_secretsmanager_secret", "arn", true))
        }
        ("aws_msk_single_scram_secret_association", "secret_arn") => {
            Some(("aws_secretsmanager_secret", "arn", false))
        }
        ("aws_msk_vpc_connection", "vpc_id") => Some(("aws_vpc", "id", false)),
        _ => None,
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn at<'a>(value: &'a Value, unknown: &'a Value, path: &[&str]) -> Option<(&'a Value, &'a Value)> {
    if path.is_empty() {
        return Some((value, unknown));
    }
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        let mask = match unknown {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(masks) if masks.len() == 1 => &masks[0],
            _ => return None,
        };
        return at(value, mask, path);
    }
    if !value.is_object() || !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
    {
        return None;
    }
    at(&value[path[0]], &unknown[path[0]], &path[1..])
}

fn scalar<'a>(value: &'a Value, unknown: &'a Value, path: &str) -> Option<&'a str> {
    let (value, mask) = at(value, unknown, &path.split('.').collect::<Vec<_>>())?;
    matches!(mask, Value::Null | Value::Bool(false))
        .then(|| value.as_str().filter(|s| !s.is_empty()))
        .flatten()
}

fn revision(value: &Value, unknown: &Value, path: &str) -> Option<u64> {
    let (value, mask) = at(value, unknown, &path.split('.').collect::<Vec<_>>())?;
    matches!(mask, Value::Null | Value::Bool(false))
        .then(|| value.as_u64().filter(|n| *n > 0))
        .flatten()
}

pub(super) fn values<'a>(
    value: &'a Value,
    unknown: &'a Value,
    path: &str,
    collection: bool,
) -> Option<BTreeSet<&'a str>> {
    if !collection {
        return scalar(value, unknown, path).map(|s| [s].into());
    }
    let (value, mask) = at(value, unknown, &path.split('.').collect::<Vec<_>>())?;
    let values = value.as_array()?;
    if values.is_empty()
        || (!matches!(mask, Value::Null | Value::Bool(false))
            && !mask.as_array().is_some_and(|m| {
                m.len() == values.len()
                    && m.iter()
                        .all(|m| matches!(m, Value::Null | Value::Bool(false)))
            }))
    {
        return None;
    }
    let unique = values
        .iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()))
        .collect::<Option<BTreeSet<_>>>()?;
    (unique.len() == values.len()).then_some(unique)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    let node = &raw.nodes[reference.target];
    reference.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            false,
        )
    })
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let Some((kind, field, collection)) = rule(&node.resource_type, &reference.attribute) else {
        return false;
    };
    if !current(node)
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || reference.sources.is_empty()
        || (!collection && reference.sources.len() != 1)
        || reference.source_attributes.len() != 1
        || !reference.source_attributes.contains(field)
    {
        return false;
    }
    let (value, unknown) = evidence[reference.target];
    let Some(actual) = values(value, unknown, &reference.attribute, collection) else {
        return false;
    };
    let mut expected = BTreeSet::new();
    for &source in &reference.sources {
        let endpoint = &nodes[source];
        if !current(endpoint)
            || endpoint.resource_type != kind
            || !super::eventbridge::bindings(
                node.provider_configuration.as_ref(),
                endpoint.provider_configuration.as_ref(),
                super::scope::scope(node)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                super::scope::scope(endpoint)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                false,
            )
        {
            return false;
        }
        let (target, mask) = evidence[source];
        let Some(identifier) = scalar(target, mask, field) else {
            return false;
        };
        if !expected.insert(identifier) {
            return false;
        }
        let revision_path = match reference.attribute.as_str() {
            "configuration_info.arn" => Some("configuration_info.revision"),
            "configuration.id" => Some("configuration.revision"),
            _ => None,
        };
        if node.resource_type == "aws_mq_broker" && reference.attribute == "configuration.id" {
            let engine = scalar(value, unknown, "engine_type");
            if !matches!(engine, Some("ActiveMQ" | "RabbitMQ"))
                || engine != scalar(target, mask, "engine_type")
            {
                return false;
            }
        }
        if let Some(path) = revision_path {
            let selected = revision(value, unknown, path);
            if selected.is_none()
                || selected != revision(target, mask, "latest_revision")
                || !refs.iter().any(|r| {
                    r.target == reference.target
                        && r.attribute == path
                        && r.complete
                        && r.instances_explicit
                        && r.issues.is_empty()
                        && r.sources.as_slice() == [source]
                        && r.source_attributes.len() == 1
                        && r.source_attributes.contains("latest_revision")
                })
            {
                return false;
            }
        }
    }
    actual == expected
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let proofs = references
        .iter()
        .enumerate()
        .filter(|(_, r)| label(&nodes[r.target].resource_type, &r.attribute).is_some())
        .map(|(i, r)| (i, verify(nodes, references, r, evidence)))
        .collect::<Vec<_>>();
    for (i, proof) in proofs {
        references[i].service_reference_verified = proof;
    }
}

pub(super) fn vpc(raw: &Input, source: usize) -> Option<usize> {
    let r = raw
        .attributes
        .iter()
        .find(|r| r.target == source && r.attribute == "vpc_id")?;
    let [parent] = r.sources.as_slice() else {
        return None;
    };
    let node = &raw.nodes[*parent];
    (r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && r.scalar_id_matches
        && r.source_attributes.len() == 1
        && r.source_attributes.contains("id")
        && node.provider.is_aws()
        && node.resource_type == "aws_vpc"
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
        && bindings(raw, r))
    .then_some(*parent)
}

fn placement(raw: &Input, target: usize) -> Option<(usize, Vec<usize>)> {
    let node = &raw.nodes[target];
    let (subnets, groups) = network(&node.resource_type)?;
    let proof = |attribute: &str| {
        raw.attributes
            .iter()
            .find(|r| r.target == target && r.attribute == attribute)
            .filter(|r| r.service_reference_verified && bindings(raw, r))
    };
    let (subnets, groups) = proof(subnets).zip(proof(groups))?;
    let members = subnets
        .sources
        .iter()
        .chain(&groups.sources)
        .copied()
        .collect::<Vec<_>>();
    let parents = members
        .iter()
        .map(|&i| vpc(raw, i))
        .collect::<Option<BTreeSet<_>>>()?;
    if parents.len() != 1 {
        return None;
    }
    let parent = *parents.first().expect("one VPC");
    if node.resource_type == "aws_msk_vpc_connection"
        && proof("vpc_id").is_none_or(|r| r.sources.as_slice() != [parent])
    {
        return None;
    }
    Some((parent, members))
}

pub(super) fn diagnostics(raw: &Input) -> Vec<crate::model::Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter_map(|(target, node)| {
            let (subnets, _) = network(&node.resource_type)?;
            (node.provider.is_aws()
                && node.mode == EntityMode::Managed
                && node.deposed_key.is_none()
                && placement(raw, target).is_none())
            .then(|| crate::model::Diagnostic {
                address: node.address.clone(),
                deposed_key: None,
                attribute: subnets.into(),
                reason: crate::model::DiagnosticReason::UnprovenEndpointValue,
            })
        })
        .collect()
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        let Some((parent, members)) = placement(raw, target) else {
            continue;
        };
        let mut evidence = Vec::new();
        for member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        if node.resource_type == "aws_msk_vpc_connection" {
            evidence.push(relationships::reference(graph, parent, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        if let Some(edge) = graph
            .edges
            .iter_mut()
            .find(|e| e.from == parent && e.to == target && e.kind == EdgeKind::Dependency)
        {
            edge.kind = EdgeKind::Containment;
        } else {
            graph.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((parent, target))
            });
        }
    }
}
