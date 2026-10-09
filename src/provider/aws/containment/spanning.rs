use crate::model::{Edge, EdgeKind, EntityMode, Graph, ResourceRole};
use crate::semantic::Input;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../../tests/unit/semantic/spanning.rs"]
mod tests;

fn ancestry(node: usize, parents: &[BTreeSet<usize>]) -> Option<Vec<usize>> {
    let mut chain = Vec::new();
    let mut visited = BTreeSet::new();
    let mut current = node;
    loop {
        if !visited.insert(current) {
            return None;
        }
        chain.push(current);
        match parents[current].len() {
            0 => return Some(chain),
            1 => current = *parents[current].first()?,
            _ => return None,
        }
    }
}

pub(super) fn common_parent(nodes: &[usize], parents: &[BTreeSet<usize>]) -> Option<usize> {
    let chains: Vec<_> = nodes
        .iter()
        .map(|&node| ancestry(node, parents))
        .collect::<Option<_>>()?;
    chains
        .first()?
        .iter()
        .copied()
        .find(|candidate| chains.iter().all(|chain| chain.contains(candidate)))
}

pub(super) fn infer(raw: &Input, mut graph: Graph) -> Graph {
    let mut parents = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Containment {
            parents[edge.to].insert(edge.from);
        }
    }
    for reference in &raw.attributes {
        let node = &raw.nodes[reference.target];
        if node.mode != EntityMode::Managed
            || !node.provider.is_aws()
            || !reference.complete
            || reference.sources.is_empty()
            || !parents[reference.target].is_empty()
        {
            continue;
        }
        match (node.resource_type.as_str(), reference.attribute.as_str()) {
            ("aws_lb", "subnets") | ("aws_ecs_service", "network_configuration.subnets") => {}
            _ => continue,
        }
        if !reference.sources.iter().all(|&source| {
            source != reference.target
                && raw.nodes[source].resource_type == "aws_subnet"
                && raw.nodes[source].role == ResourceRole::Container
                && graph.edges.iter().any(|edge| {
                    edge.from == source
                        && edge.to == reference.target
                        && edge.kind == EdgeKind::Dependency
                })
        }) {
            continue;
        }
        let parent = common_parent(&reference.sources, &parents).filter(|&parent| {
            parent != reference.target && raw.nodes[parent].role == ResourceRole::Container
        });
        for edge in &mut graph.edges {
            if edge.to == reference.target && edge.kind == EdgeKind::Dependency {
                match (
                    Some(edge.from) == parent,
                    reference.sources.contains(&edge.from),
                ) {
                    (true, _) => edge.kind = EdgeKind::Containment,
                    (false, true) => edge.kind = EdgeKind::Connection,
                    _ => {}
                }
            }
        }
        if let Some(parent) = parent {
            if !graph.edges.iter().any(|edge| {
                edge.from == parent
                    && edge.to == reference.target
                    && edge.kind == EdgeKind::Containment
            }) {
                graph.edges.push(Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((parent, reference.target))
                });
            }
        }
    }
    graph
}
