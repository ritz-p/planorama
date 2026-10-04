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

pub struct Layout<'a> {
    pub header_heights: Vec<usize>,
    pub containers: Vec<(usize, Bounds)>,
    pub parents: Vec<Option<usize>>,
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

impl<'a> Layout<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        if graph
            .nodes
            .iter()
            .any(|node| node.role == crate::model::ResourceRole::Container)
        {
            return containers::place(graph);
        }
        let ranks = rank::compute(graph);
        let mut placement = placement::place(graph, &ranks);
        let routed = routing::route(graph, &ranks, &mut placement.positions, &placement.channels);
        Self {
            header_heights: vec![NODE_HEIGHT; graph.nodes.len()],
            containers: Vec::new(),
            parents: vec![None; graph.nodes.len()],
            width: routed.width,
            height: placement.height,
            positions: placement.positions,
            bands: placement.bands,
            paths: routed.paths,
            junctions: routed.junctions,
        }
    }
}
