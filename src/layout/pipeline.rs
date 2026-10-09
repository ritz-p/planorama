//! Common hierarchy -> placement -> geometry -> routing stages.
use super::{Bounds, ContainmentTree, Layout, NODE_HEIGHT, containers, placement, rank, routing};
use crate::model::{Graph, ResourceRole};

enum Strategy {
    Flat {
        ranks: Vec<usize>,
        channels: Vec<usize>,
    },
    Nested,
}
pub(super) struct Placed<'a> {
    pub layout: Layout<'a>,
    strategy: Strategy,
}

pub(super) fn place(graph: &Graph, tree: ContainmentTree) -> Placed<'_> {
    if graph
        .nodes
        .iter()
        .any(|node| node.role == ResourceRole::Container)
    {
        Placed {
            layout: containers::place_geometry(graph, tree, true),
            strategy: Strategy::Nested,
        }
    } else {
        let ranks = rank::compute(graph);
        let placement = placement::place(graph, &ranks);
        let bounds = placement
            .positions
            .iter()
            .copied()
            .map(Bounds::card)
            .collect();
        Placed {
            layout: Layout {
                bounds,
                header_heights: vec![NODE_HEIGHT; graph.nodes.len()],
                containers: Vec::new(),
                containment: tree,
                width: 1040,
                height: placement.height,
                positions: placement.positions,
                bands: placement.bands,
                paths: Vec::new(),
                junctions: Vec::new(),
            },
            strategy: Strategy::Flat {
                ranks,
                channels: placement.channels,
            },
        }
    }
}

pub(super) fn route(graph: &Graph, placed: &mut Placed<'_>) {
    match &placed.strategy {
        Strategy::Flat { ranks, channels } => {
            let routed = routing::route(graph, ranks, &mut placed.layout.bounds, channels);
            placed.layout.width = routed.width;
            placed.layout.paths = routed.paths;
            placed.layout.junctions = routed.junctions;
        }
        Strategy::Nested => containers::route(graph, &mut placed.layout, true),
    }
    placed.layout.positions = placed
        .layout
        .bounds
        .iter()
        .map(|bounds| bounds.origin)
        .collect();
}
