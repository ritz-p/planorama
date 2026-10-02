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

pub struct Layout<'a> {
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    pub paths: Vec<Vec<Point>>,
}

impl<'a> Layout<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        let ranks = rank::compute(graph);
        let placement = placement::place(graph, &ranks);
        let paths = routing::route(graph, &ranks, &placement.positions, &placement.channels);
        Self {
            width: placement.width,
            height: placement.height,
            positions: placement.positions,
            bands: placement.bands,
            paths,
        }
    }
}
