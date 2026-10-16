use crate::model::{
    AttributeReference, DiagnosticReason, Edge, EdgeKind, EntityMode, Graph, ResourceRole,
};
use crate::semantic::Input;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../../tests/unit/semantic/indirect.rs"]
mod tests;

fn static_reference<'a>(
    raw: &'a Input,
    target: usize,
    attribute: &str,
) -> Option<&'a AttributeReference> {
    raw.attributes
        .iter()
        .find(|r| r.target == target && r.attribute == attribute)
        .filter(|r| {
            r.complete
                && !r.sources.is_empty()
                && !r
                    .issues
                    .contains(&DiagnosticReason::DynamicInstanceSelection)
        })
}

pub(in crate::provider::aws) fn resolved_subnets(raw: &Input, target: usize) -> Option<&[usize]> {
    let (attribute, group_type) = group_rule(&raw.nodes[target].resource_type)?;
    let group_ref = static_reference(raw, target, attribute)?;
    let [group] = group_ref.sources.as_slice() else {
        return None;
    };
    let group_node = &raw.nodes[*group];
    if group_node.resource_type != group_type || !group_node.provider.is_aws() {
        return None;
    }
    let subnets = static_reference(raw, *group, "subnet_ids")?;
    subnets
        .sources
        .iter()
        .all(|&source| {
            raw.nodes[source].resource_type == "aws_subnet"
                && raw.nodes[source].provider.is_aws()
                && raw.nodes[source].role == ResourceRole::Container
        })
        .then_some(subnets.sources.as_slice())
}

fn group_rule(resource_type: &str) -> Option<(&'static str, &'static str)> {
    match resource_type {
        "aws_db_instance" | "aws_rds_cluster" => {
            Some(("db_subnet_group_name", "aws_db_subnet_group"))
        }
        "aws_elasticache_replication_group" => {
            Some(("subnet_group_name", "aws_elasticache_subnet_group"))
        }
        _ => None,
    }
}

pub(super) fn infer(raw: &Input, mut graph: Graph) -> Graph {
    let mut parents = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Containment {
            parents[edge.to].insert(edge.from);
        }
    }
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.mode != EntityMode::Managed
            || !node.provider.is_aws()
            || group_rule(&node.resource_type).is_none()
            || !parents[target].is_empty()
        {
            continue;
        }
        let Some(subnets) = resolved_subnets(raw, target) else {
            continue;
        };
        let (attribute, _) = group_rule(&node.resource_type).expect("supported subnet group rule");
        let group = static_reference(raw, target, attribute)
            .expect("resolved subnet group")
            .sources[0];
        let parent = if node.resource_type == "aws_elasticache_replication_group" {
            (parents[group].len() == 1)
                .then(|| *parents[group].first().expect("one parent"))
                .filter(|&parent| raw.nodes[parent].resource_type == "aws_vpc")
        } else {
            super::spanning::common_parent(subnets, &parents)
        };
        let Some(parent) = parent else {
            continue;
        };
        if parent == target || raw.nodes[parent].role != ResourceRole::Container {
            continue;
        }
        let mut proof = super::spanning::evidence(&graph, parent, subnets, &parents);
        proof.extend(
            subnets
                .iter()
                .map(|&source| crate::semantic::relationships::reference(&graph, source, group)),
        );
        proof.push(crate::semantic::relationships::reference(
            &graph, group, target,
        ));
        crate::semantic::relationships::record(
            &mut graph,
            parent,
            target,
            EdgeKind::Containment,
            proof,
        );
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
    graph
}
