mod geometry;
pub(crate) use geometry::{Bounds, Point, Port, Side};
mod containers;
mod containment;
pub(crate) use containment::ContainmentTree;
#[cfg(test)]
#[path = "../tests/support/layout_metrics.rs"]
pub(crate) mod metrics;
mod pipeline;
mod placement;
mod rank;
pub(crate) mod relationship_markers;
mod routing;
mod routing_shared;
mod scopes;
mod spatial;
#[cfg(test)]
#[path = "../tests/unit/layout.rs"]
mod tests;

use crate::model::architecture::Graph;

pub(crate) fn route_to_margin(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    routing_shared::search::to_margin(start, end, obstacles)
}

pub const NODE_WIDTH: usize = 320;
pub const NODE_HEIGHT: usize = 96;

fn resource_edges(graph: &Graph) -> Vec<bool> {
    let relationships: std::collections::BTreeSet<_> = graph
        .relationships
        .iter()
        .filter(|relationship| {
            relationship.provenance.iter().any(|provenance| {
                matches!(
                    provenance,
                    crate::model::RelationshipProvenance::Resource { .. }
                )
            })
        })
        .map(|relationship| (&relationship.from, &relationship.to, relationship.kind))
        .collect();
    graph
        .edges
        .iter()
        .map(|edge| {
            relationships.contains(&(
                &graph.nodes[edge.from].entity.id,
                &graph.nodes[edge.to].entity.id,
                edge.kind,
            ))
        })
        .collect()
}

pub struct Band<'a> {
    pub label: &'a str,
    pub top: usize,
    pub height: usize,
}

pub struct Layout<'a> {
    pub scopes: Vec<ScopePanel>,
    pub header_heights: Vec<usize>,
    pub bounds: Vec<Bounds>,
    pub containers: Vec<usize>,
    pub containment: ContainmentTree,
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

pub struct ScopePanel {
    pub scope: crate::model::architecture::DeploymentScope,
    pub bounds: Bounds,
}

impl<'a> Layout<'a> {
    pub(crate) fn occludes(&self, node: usize, edge: &crate::model::Edge) -> bool {
        self.containers.contains(&node)
            && !self.containment.is_ancestor(node, edge.from)
            && !self.containment.is_ancestor(node, edge.to)
    }

    pub(crate) fn occluders(&self, edge: &crate::model::Edge) -> Vec<usize> {
        self.containers
            .iter()
            .copied()
            .filter(|&node| {
                self.occludes(node, edge)
                    && !self.containment.parents[node]
                        .is_some_and(|parent| self.occludes(parent, edge))
            })
            .collect()
    }

    pub fn new(graph: &'a Graph) -> Self {
        if graph.nodes.iter().any(|n| {
            n.placement == crate::model::Placement::Logical
                || crate::view::routes::owner(graph, n).is_some()
                || (!n.routes.entries.is_empty() && n.routes.reserved_height == 0)
        }) || graph.nodes.windows(2).any(|nodes| {
            (&nodes[0].entity.id, &nodes[0].address) > (&nodes[1].entity.id, &nodes[1].address)
        }) {
            return spatial::layout(graph);
        }
        let containment = ContainmentTree::new(graph);
        let mut placed = pipeline::place(graph, containment);
        pipeline::route(graph, &mut placed);
        placed.layout
    }
}
