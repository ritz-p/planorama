use crate::model::{
    AttributeReference, DiagnosticReason, Edge, EdgeKind, EntityMode, Graph, ResourceRole,
    TerraformGraph,
};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/indirect.rs"]
mod tests;

fn static_reference<'a>(
    raw: &'a TerraformGraph,
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

pub(super) fn infer(raw: &TerraformGraph, mut graph: Graph) -> Graph {
    let mut parents = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Containment {
            parents[edge.to].insert(edge.from);
        }
    }
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.mode != EntityMode::Managed
            || !node.provider.is_aws()
            || !matches!(
                node.resource_type.as_str(),
                "aws_db_instance" | "aws_rds_cluster"
            )
            || !parents[target].is_empty()
        {
            continue;
        }
        let Some(group_ref) = static_reference(raw, target, "db_subnet_group_name") else {
            continue;
        };
        let [group] = group_ref.sources.as_slice() else {
            continue;
        };
        let group_node = &raw.nodes[*group];
        if group_node.resource_type != "aws_db_subnet_group" || !group_node.provider.is_aws() {
            continue;
        }
        let Some(subnets) = static_reference(raw, *group, "subnet_ids") else {
            continue;
        };
        if !subnets.sources.iter().all(|&source| {
            raw.nodes[source].resource_type == "aws_subnet"
                && raw.nodes[source].provider.is_aws()
                && raw.nodes[source].role == ResourceRole::Container
        }) {
            continue;
        }
        let Some(parent) = super::spanning::common_parent(&subnets.sources, &parents) else {
            continue;
        };
        if parent == target || raw.nodes[parent].role != ResourceRole::Container {
            continue;
        }
        // Keep the group card and both dependency hops; only add the derived scope.
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
