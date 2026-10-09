use crate::model::{
    ArchitectureRelationship, EdgeKind, Graph, RelationshipProvenance, TerraformEntityId,
    TerraformPlan,
};

pub(super) fn collect(graph: &Graph, plan: &TerraformPlan) -> Vec<ArchitectureRelationship> {
    let mut forward = vec![Vec::new(); plan.nodes.len()];
    let mut backward = forward.clone();
    for edge in &plan.edges {
        forward[edge.from].push(edge.to);
        backward[edge.to].push(edge.from);
    }
    let mut relationships: Vec<_> = graph
        .edges
        .iter()
        .map(|edge| {
            let from = &graph.nodes[edge.from];
            let to = &graph.nodes[edge.to];
            let mut provenance = Vec::new();
            if let Some(change) = &edge.change {
                provenance.push(RelationshipProvenance::Resource {
                    source: TerraformEntityId {
                        address: change.address.clone(),
                        deposed_key: None,
                    },
                    change: change.clone(),
                });
            } else {
                // Retain actual Terraform references that support the inferred
                // relationship, rather than inventing a standalone helper resource.
                let start = plan
                    .nodes
                    .iter()
                    .position(|n| n.address == from.address && n.deposed_key == from.deposed_key);
                let end = plan
                    .nodes
                    .iter()
                    .position(|n| n.address == to.address && n.deposed_key == to.deposed_key);
                let downstream = reachable(start, &forward);
                let upstream = reachable(end, &backward);
                for reference in &plan.edges {
                    let source = &plan.nodes[reference.from];
                    let target = &plan.nodes[reference.to];
                    if downstream[reference.from]
                        && upstream[reference.to]
                        && (edge.kind != EdgeKind::Dependency
                            || Some(reference.from) == start && Some(reference.to) == end)
                    {
                        provenance.push(RelationshipProvenance::Reference {
                            from: source.into(),
                            to: target.into(),
                        });
                    }
                }
            }
            ArchitectureRelationship::new(
                from.entity.id.clone(),
                to.entity.id.clone(),
                edge.kind,
                edge.kind != EdgeKind::Dependency && edge.change.is_none(),
                provenance,
            )
        })
        .collect();
    relationships.sort();
    relationships.dedup();
    relationships
}

fn reachable(start: Option<usize>, adjacency: &[Vec<usize>]) -> Vec<bool> {
    let mut visited = vec![false; adjacency.len()];
    let mut pending: Vec<_> = start.into_iter().collect();
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut visited[index], true) {
            continue;
        }
        pending.extend(&adjacency[index]);
    }
    visited
}
