use crate::model::{DiagnosticReason, Edge, EdgeKind, EntityMode, Graph, ResourceRole};
use crate::semantic::{Input, relationships};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../../tests/unit/semantic/vpc_scope.rs"]
mod tests;

pub(in crate::provider::aws) fn attribute(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "aws_db_subnet_group" | "aws_elasticache_subnet_group" => Some("subnet_ids"),
        "aws_lambda_function" | "aws_eks_cluster" => Some("vpc_config.subnet_ids"),
        "aws_opensearch_domain" => Some("vpc_options.subnet_ids"),
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
    for reference in &raw.attributes {
        let target = reference.target;
        let node = &raw.nodes[target];
        if node.mode != EntityMode::Managed
            || !node.provider.is_aws()
            || attribute(&node.resource_type) != Some(reference.attribute.as_str())
            || !reference.complete
            || reference.sources.is_empty()
            || reference
                .issues
                .contains(&DiagnosticReason::DynamicInstanceSelection)
            || !parents[target].is_empty()
            || !reference.sources.iter().all(|&source| {
                raw.nodes[source].resource_type == "aws_subnet"
                    && raw.nodes[source].provider.is_aws()
                    && raw.nodes[source].role == ResourceRole::Container
            })
        {
            continue;
        }
        let Some(common) = super::spanning::common_parent(&reference.sources, &parents) else {
            continue;
        };
        let parent = if raw.nodes[common].resource_type == "aws_subnet" {
            parents[common].first().copied()
        } else {
            Some(common)
        };
        let Some(parent) = parent.filter(|&parent| {
            raw.nodes[parent].resource_type == "aws_vpc"
                && raw.nodes[parent].provider.is_aws()
                && raw.nodes[parent].role == ResourceRole::Container
        }) else {
            continue;
        };
        let mut proof = super::spanning::evidence(&graph, parent, &reference.sources, &parents);
        proof.extend(
            reference
                .sources
                .iter()
                .map(|&source| relationships::reference(&graph, source, target)),
        );
        relationships::record(&mut graph, parent, target, EdgeKind::Containment, proof);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
        parents[target].insert(parent);
    }
    graph
}
