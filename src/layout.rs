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
mod routing;
mod routing_shared;
#[cfg(test)]
#[path = "../tests/unit/layout.rs"]
mod tests;

use crate::model::architecture::Graph;

pub(crate) fn route_to_margin(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    routing_shared::search::to_margin(start, end, obstacles)
}

pub const NODE_WIDTH: usize = 320;
pub const NODE_HEIGHT: usize = 96;

pub struct Band<'a> {
    pub label: &'a str,
    pub top: usize,
    pub height: usize,
}

pub struct Layout<'a> {
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

impl<'a> Layout<'a> {
    pub fn component_bounds<'b>(
        &self,
        graph: &'b Graph,
    ) -> Vec<(&'b crate::model::architecture::ArchitectureId, Bounds)> {
        let mut top = self.height + if graph.checks.is_empty() { 0 } else { 24 };
        graph
            .components
            .iter()
            .map(|component| {
                let height = 56 + component.entity.provenance.len() * 24;
                let bounds = Bounds {
                    origin: Point { x: 35, y: top },
                    width: self.width - 70,
                    height,
                };
                top += height + 20;
                (&component.entity.id, bounds)
            })
            .collect()
    }
    pub fn new(graph: &'a Graph) -> Self {
        let containment = ContainmentTree::new(graph);
        let mut placed = pipeline::place(graph, containment);
        pipeline::route(graph, &mut placed);
        placed.layout
    }
}
