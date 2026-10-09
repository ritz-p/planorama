use crate::model::{Edge, EdgeChange, EdgeKind, EntityMode, Graph, ResourceRole};
use crate::semantic::Input;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/associations.rs"]
mod tests;

pub(super) fn rule(resource_type: &str) -> Option<[(&'static str, &'static str); 2]> {
    match resource_type {
        "aws_route_table_association" => Some([
            ("subnet_id", "aws_subnet"),
            ("route_table_id", "aws_route_table"),
        ]),
        "aws_lb_target_group_attachment" => Some([
            ("target_group_arn", "aws_lb_target_group"),
            ("target_id", "aws_instance"),
        ]),
        "aws_vpc_endpoint_route_table_association" => Some([
            ("vpc_endpoint_id", "aws_vpc_endpoint"),
            ("route_table_id", "aws_route_table"),
        ]),
        _ => None,
    }
}

pub(super) fn lower(raw: &Input) -> Graph {
    let mut incident_edges = vec![Vec::new(); raw.nodes.len()];
    for edge in &raw.edges {
        incident_edges[edge.from].push(edge);
        if edge.from != edge.to {
            incident_edges[edge.to].push(edge);
        }
    }
    let mut replacements = BTreeMap::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if (node.role != ResourceRole::Association && node.resource_type != "aws_route")
            || !node.provider.is_aws()
            || node.mode != EntityMode::Managed
            || node.deposed_key.is_some()
        {
            continue;
        }
        let endpoints = if node.resource_type == "aws_route" {
            // Invalid routes stay visible; --diagnostics exposes the same failure.
            super::routes::endpoints(raw, index).ok()
        } else {
            rule(&node.resource_type).and_then(
                |[(from_attribute, from_type), (to_attribute, to_type)]| {
                    endpoint(raw, index, from_attribute, from_type).zip(endpoint(
                        raw,
                        index,
                        to_attribute,
                        to_type,
                    ))
                },
            )
        };
        if let Some((from, to)) = endpoints {
            let incident = &incident_edges[index];
            if can_lower(index, from, to, incident) {
                replacements.insert(
                    index,
                    Edge {
                        from,
                        to,
                        kind: EdgeKind::Association,
                        change: Some(EdgeChange {
                            address: node.address.clone(),
                            previous_address: node.previous_address.clone(),
                            metadata: node.metadata.clone(),
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
    // Evaluate every rule against the same graph, then remove helpers and remap
    // once. This prevents one lowering from enabling another through lost edges.
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
    Graph {
        relationships: Vec::new(),
        components: raw.components.clone(),
        checks: raw.checks.clone(),
        status: raw.status,
        nodes,
        edges,
    }
}

pub(super) fn can_lower(index: usize, from: usize, to: usize, incident: &[&Edge]) -> bool {
    incident.len() == 2
        && incident.iter().all(|e| {
            e.to == index && e.kind == EdgeKind::Dependency && (e.from == from || e.from == to)
        })
        && incident.iter().any(|e| e.from == from)
        && incident.iter().any(|e| e.from == to)
}

pub(super) fn endpoint(
    raw: &Input,
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
                && !reference
                    .issues
                    .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
                && *source != target
                && raw.nodes[*source].resource_type == resource_type =>
        // Provider identity prevents custom lookalike types becoming AWS endpoints.
        {
            raw.nodes[*source].provider.is_aws().then_some(*source)
        }
        _ => None,
    }
}
