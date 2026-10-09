mod containers;
#[cfg(test)]
#[path = "../tests/support/layout_metrics.rs"]
pub(crate) mod metrics;
mod placement;
mod rank;
mod routing;
#[cfg(test)]
#[path = "../tests/unit/layout.rs"]
mod tests;

use crate::model::Graph;

pub fn route_to_margin(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    containers::to_margin(start, end, obstacles)
}

pub const NODE_WIDTH: usize = 320;
pub const NODE_HEIGHT: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

pub struct Band<'a> {
    pub label: &'a str,
    pub top: usize,
    pub height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub origin: Point,
    pub width: usize,
    pub height: usize,
}

impl Bounds {
    pub fn card(origin: Point) -> Self {
        Self {
            origin,
            width: NODE_WIDTH,
            height: NODE_HEIGHT,
        }
    }

    pub fn right(self) -> usize {
        self.origin.x + self.width
    }

    pub fn header(self, height: usize) -> Self {
        Self { height, ..self }
    }
}

pub struct Layout<'a> {
    pub header_heights: Vec<usize>,
    pub bounds: Vec<Bounds>,
    pub containers: Vec<usize>,
    pub parents: Vec<Option<usize>>,
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

impl<'a> Layout<'a> {
    /// Logical panels are positioned using architecture IDs and member counts;
    /// no Terraform address or spatial-container role is required.
    pub fn component_bounds<'b>(
        &self,
        graph: &'b Graph,
    ) -> Vec<(&'b crate::model::ArchitectureId, Bounds)> {
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
        if graph
            .nodes
            .iter()
            .any(|node| node.role == crate::model::ResourceRole::Container)
        {
            return containers::place(graph);
        }
        let ranks = rank::compute(graph);
        let placement = placement::place(graph, &ranks);
        let mut bounds: Vec<_> = placement.positions.into_iter().map(Bounds::card).collect();
        let routed = routing::route(graph, &ranks, &mut bounds, &placement.channels);
        let positions = bounds.iter().map(|bounds| bounds.origin).collect();
        Self {
            bounds,
            header_heights: vec![NODE_HEIGHT; graph.nodes.len()],
            containers: Vec::new(),
            parents: vec![None; graph.nodes.len()],
            width: routed.width,
            height: placement.height,
            positions,
            bands: placement.bands,
            paths: routed.paths,
            junctions: routed.junctions,
        }
    }
}
