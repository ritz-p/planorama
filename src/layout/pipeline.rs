use super::{Bounds, ContainmentTree, Layout, NODE_HEIGHT, containers, placement, rank, routing};
use crate::model::architecture::{Graph, ResourceRole};

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
    if graph.nodes.iter().any(|node| {
        node.role == ResourceRole::Container
            || node
                .entity
                .scope
                .as_ref()
                .is_some_and(|s| s.region.is_some())
    }) {
        Placed {
            layout: containers::place_geometry(graph, tree, true),
            strategy: Strategy::Nested,
        }
    } else {
        let ranks = rank::compute(graph);
        let indexed = super::resource_edges(graph)
            .into_iter()
            .any(|indexed| indexed);
        let header_heights = if indexed {
            containers::header_heights(graph, &tree)
        } else {
            vec![NODE_HEIGHT; graph.nodes.len()]
        };
        let placement = if indexed {
            placement::place_with_heights(graph, &ranks, &header_heights)
        } else {
            placement::place(graph, &ranks)
        };
        let bounds = placement
            .positions
            .iter()
            .copied()
            .zip(&header_heights)
            .map(|(origin, &height)| Bounds {
                height,
                ..Bounds::card(origin)
            })
            .collect::<Vec<_>>();
        let width = bounds
            .iter()
            .map(|b| b.right() + 80)
            .max()
            .unwrap_or(1040)
            .max(1040);
        Placed {
            layout: Layout {
                scopes: Vec::new(),
                bounds,
                header_heights,
                containers: Vec::new(),
                containment: tree,
                width,
                height: placement.height,
                positions: placement.positions,
                bands: placement.bands,
                paths: Vec::new(),
                junctions: Vec::new(),
            },
            strategy: if indexed {
                Strategy::Nested
            } else {
                Strategy::Flat {
                    ranks,
                    channels: placement.channels,
                }
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
