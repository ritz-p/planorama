use crate::model::{Edge, EdgeChange, EdgeKind, EntityMode, Graph, ResourceRole, TerraformGraph};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/semantic/associations.rs"]
mod tests;

pub(super) fn lower(raw: &TerraformGraph) -> Graph {
    let mut incident_edges = vec![Vec::new(); raw.nodes.len()];
    for edge in &raw.edges {
        incident_edges[edge.from].push(edge);
        if edge.from != edge.to {
            incident_edges[edge.to].push(edge);
        }
    }
    let mut replacements = BTreeMap::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if node.role != ResourceRole::Association
            || node.resource_type != "aws_route_table_association"
            || node.mode != EntityMode::Managed
        {
            continue;
        }
        let endpoints = endpoint(raw, index, "subnet_id", "aws_subnet").zip(endpoint(
            raw,
            index,
            "route_table_id",
            "aws_route_table",
        ));
        if let Some((from, to)) = endpoints {
            let incident = &incident_edges[index];
            if incident.len() == 2
                && incident.iter().all(|e| {
                    e.to == index
                        && e.kind == EdgeKind::Dependency
                        && (e.from == from || e.from == to)
                })
                && incident.iter().any(|e| e.from == from)
                && incident.iter().any(|e| e.from == to)
            {
                replacements.insert(
                    index,
                    Edge {
                        from,
                        to,
                        kind: EdgeKind::Association,
                        change: Some(EdgeChange {
                            address: node.address.clone(),
                            action: node.action,
                        }),
                    },
                );
            }
        }
    }
    if replacements.is_empty() {
        return raw.graph.clone();
    }
    let mut indices = vec![None; raw.nodes.len()];
    let mut nodes = Vec::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if !replacements.contains_key(&index) {
            indices[index] = Some(nodes.len());
            nodes.push(node.clone());
        }
    }
    let mut edges: Vec<_> = raw
        .edges
        .iter()
        .chain(replacements.values())
        .filter_map(|edge| {
            indices[edge.from]
                .zip(indices[edge.to])
                .map(|(from, to)| Edge {
                    from,
                    to,
                    ..edge.clone()
                })
        })
        .collect();
    edges.sort();
    edges.dedup();
    Graph { nodes, edges }
}

fn endpoint(
    raw: &TerraformGraph,
    target: usize,
    attribute: &str,
    resource_type: &str,
) -> Option<usize> {
    let reference = raw
        .attributes
        .iter()
        .find(|a| a.target == target && a.attribute == attribute)?;
    match reference.sources.as_slice() {
        [source]
            if reference.complete
                && *source != target
                && raw.nodes[*source].resource_type == resource_type =>
        {
            Some(*source)
        }
        _ => None,
    }
}
