use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/dax.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_dax_cluster", "subnet_group_name") => Some("DAX subnet group"),
        ("aws_dax_subnet_group", "subnet_ids") => Some("Configured subnets"),
        ("aws_dax_cluster", "security_group_ids") => Some("Configured security groups"),
        ("aws_dax_cluster", "parameter_group_name") => Some("DAX parameter group"),
        ("aws_dax_cluster", "iam_role_arn") => Some("DAX IAM role"),
        ("aws_dax_cluster", "notification_topic_arn") => Some("DAX notification topic"),
        _ => None,
    }
}

fn fields(resource: &str, attribute: &str, endpoint: &str) -> &'static [&'static str] {
    match (resource, attribute, endpoint) {
        ("aws_dax_cluster", "subnet_group_name", "aws_dax_subnet_group")
        | ("aws_dax_cluster", "parameter_group_name", "aws_dax_parameter_group") => &["name", "id"],
        ("aws_dax_subnet_group", "subnet_ids", "aws_subnet")
        | ("aws_dax_cluster", "security_group_ids", "aws_security_group") => &["id"],
        ("aws_dax_cluster", "iam_role_arn", "aws_iam_role")
        | ("aws_dax_cluster", "notification_topic_arn", "aws_sns_topic") => &["arn"],
        _ => &[],
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&source| {
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
            endpoint.resource_type == "aws_iam_role",
        )
    })
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[r.target];
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    if !current(node)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
        || r.source_attributes.len() != 1
    {
        return false;
    }
    let collection = matches!(r.attribute.as_str(), "subnet_ids" | "security_group_ids");
    if !collection && r.sources.len() != 1 {
        return false;
    }
    let (value, unknown) = evidence[r.target];
    let Some(actual) = super::streaming::values(value, unknown, &r.attribute, collection) else {
        return false;
    };
    let field = r.source_attributes.first().expect("one attribute");
    let mut expected = BTreeSet::new();
    for &source in &r.sources {
        let endpoint = &nodes[source];
        if !current(endpoint)
            || !fields(&node.resource_type, &r.attribute, &endpoint.resource_type)
                .contains(&field.as_str())
        {
            return false;
        }
        let Some(value) = super::service_evidence::known(evidence, source, field) else {
            return false;
        };
        if !expected.insert(value) {
            return false;
        }
    }
    actual == expected
}

fn placement(raw: &Input, target: usize) -> Option<(usize, usize, Vec<usize>)> {
    if raw.nodes[target].resource_type != "aws_dax_cluster" {
        return None;
    }
    let proof = |target, attribute| {
        raw.attributes
            .iter()
            .find(|r| r.target == target && r.attribute == attribute)
            .filter(|r| r.service_reference_verified && bindings(raw, r))
    };
    let group = proof(target, "subnet_group_name")?;
    let [group] = group.sources.as_slice() else {
        return None;
    };
    let subnets = proof(*group, "subnet_ids")?;
    let groups = proof(target, "security_group_ids")?;
    let members: Vec<_> = subnets
        .sources
        .iter()
        .chain(&groups.sources)
        .copied()
        .collect();
    let parents = members
        .iter()
        .map(|&i| super::streaming::vpc(raw, i))
        .collect::<Option<BTreeSet<_>>>()?;
    if parents.len() != 1 {
        return None;
    }
    Some((*parents.first()?, *group, members))
}

pub(super) fn diagnostics(raw: &Input) -> Vec<crate::model::Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter(|(target, node)| {
            node.resource_type == "aws_dax_cluster"
                && node.provider.is_aws()
                && node.mode == EntityMode::Managed
                && node.deposed_key.is_none()
                && placement(raw, *target).is_none()
        })
        .map(|(_, node)| crate::model::Diagnostic {
            address: node.address.clone(),
            deposed_key: None,
            attribute: "subnet_group_name".into(),
            reason: crate::model::DiagnosticReason::UnprovenEndpointValue,
        })
        .collect()
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for target in 0..raw.nodes.len() {
        let Some((parent, group, members)) = placement(raw, target) else {
            continue;
        };
        let mut evidence = vec![relationships::reference(graph, group, target)];
        for member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(
                graph,
                member,
                if raw.nodes[member].resource_type == "aws_subnet" {
                    group
                } else {
                    target
                },
            ));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
