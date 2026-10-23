use super::{Bounds, ContainmentTree, Layout, containers, placement, rank, routing};
use crate::model::architecture::{Graph, ResourceRole};

enum Strategy {
    Flat {
        ranks: Vec<usize>,
        channels: Vec<usize>,
    },
    Nested,
}

#[cfg(test)]
#[path = "../../tests/unit/layout/pipeline.rs"]
mod tests;

pub(super) struct Placed<'a> {
    pub layout: Layout<'a>,
    strategy: Strategy,
}

pub(super) async fn place_step(graph: &Graph, tree: ContainmentTree) -> Placed<'_> {
    crate::work::checkpoint().await;
    place_sized_step(graph, tree, None).await
}

#[cfg(test)]
fn place_sized(graph: &Graph, tree: ContainmentTree, heights: Option<Vec<usize>>) -> Placed<'_> {
    crate::work::complete(place_sized_step(graph, tree, heights))
}

async fn place_sized_step(
    graph: &Graph,
    tree: ContainmentTree,
    heights: Option<Vec<usize>>,
) -> Placed<'_> {
    crate::work::checkpoint().await;
    let mut placed = if graph.nodes.iter().any(|node| {
        node.role == ResourceRole::Container
            || node
                .entity
                .scope
                .as_ref()
                .is_some_and(|s| s.region.is_some() || s.account.is_some())
    }) {
        Placed {
            layout: if let Some(heights) = heights {
                containers::place_with_heights(graph, tree, true, heights)
            } else {
                containers::place_geometry(graph, tree, true)
            },
            strategy: Strategy::Nested,
        }
    } else {
        let ranks = rank::compute(graph);
        let indexed = super::resource_edges(graph)
            .into_iter()
            .any(|indexed| indexed);
        let header_heights = heights.unwrap_or_else(|| {
            if indexed {
                containers::header_heights(graph, &tree)
            } else {
                graph
                    .nodes
                    .iter()
                    .map(|n| crate::view::card::measure(n).height)
                    .collect()
            }
        });
        let placement = placement::place_with_heights(graph, &ranks, &header_heights);
        let bounds = placement
            .positions
            .iter()
            .copied()
            .zip(&header_heights)
            .zip(&graph.nodes)
            .map(|((origin, &height), node)| Bounds {
                origin,
                height,
                width: crate::view::card::width(node),
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
            strategy: if indexed || !crate::provider::aws::owner_affinities(graph).is_empty() {
                Strategy::Nested
            } else {
                Strategy::Flat {
                    ranks,
                    channels: placement.channels,
                }
            },
        }
    };
    super::locality::refine_step(graph, &mut placed.layout).await;
    placed
}

pub(super) async fn route_step<'a>(graph: &'a Graph, placed: &mut Placed<'a>) {
    crate::work::checkpoint().await;
    match &placed.strategy {
        Strategy::Flat { ranks, channels } => {
            let routed =
                routing::route_step(graph, ranks, &mut placed.layout.bounds, channels).await;
            placed.layout.width = routed.width;
            placed.layout.paths = routed.paths;
            placed.layout.junctions = routed.junctions;
        }
        Strategy::Nested => containers::route_step(graph, &mut placed.layout, true).await,
    }
    refine_step(graph, placed).await;
    super::routing_shared::refinement::refine_step(graph, &mut placed.layout).await;
}

#[cfg(test)]
fn refine<'a>(graph: &'a Graph, placed: &mut Placed<'a>) {
    crate::work::complete(refine_step(graph, placed))
}

async fn refine_step<'a>(graph: &'a Graph, placed: &mut Placed<'a>) {
    crate::work::checkpoint().await;
    let side = |a: super::Point, b: super::Point| match a.x.cmp(&b.x) {
        std::cmp::Ordering::Equal => {
            if a.y > b.y {
                super::Side::Top
            } else {
                super::Side::Bottom
            }
        }
        std::cmp::Ordering::Greater => super::Side::Left,
        std::cmp::Ordering::Less => super::Side::Right,
    };
    let mut sides: Vec<_> = placed
        .layout
        .paths
        .iter()
        .map(|path| {
            if path.len() < 2 {
                [super::Side::Right; 2]
            } else {
                [
                    side(path[0], path[1]),
                    side(path[path.len() - 1], path[path.len() - 2]),
                ]
            }
        })
        .collect();
    let visible: Vec<_> = placed.layout.paths.iter().map(|p| p.len() >= 2).collect();
    let sides_changed = super::routing_shared::slots::fit_sides(
        graph,
        &placed.layout.bounds,
        &mut sides,
        &visible,
        Some(&placed.layout.paths),
    );
    let demand = super::routing_shared::slots::required_heights(graph, &sides, &visible);
    let mut heights: Vec<_> = demand
        .iter()
        .enumerate()
        .map(|(node, &height)| {
            let inferred = placed.layout.containment.parents[node]
                .is_some_and(|parent| graph.derived_containment(parent, node));
            height.max(
                crate::view::card::measure(&graph.nodes[node]).height
                    + if inferred { 18 } else { 0 },
            )
        })
        .collect();
    if sides_changed
        || heights != placed.layout.header_heights
        || super::routing_shared::slots::crowded(graph, &placed.layout.paths, &sides)
    {
        super::routing_shared::slots::fit_sides(
            graph,
            &placed.layout.bounds,
            &mut sides,
            &visible,
            None,
        );
        let independent_demand =
            super::routing_shared::slots::required_heights(graph, &sides, &visible);
        for (height, demand) in heights.iter_mut().zip(independent_demand) {
            crate::work::checkpoint().await;
            *height = (*height).max(demand);
        }
        *placed = place_sized_step(graph, ContainmentTree::new(graph), Some(heights)).await;
        match &placed.strategy {
            Strategy::Flat { ranks, channels } => {
                let routed = routing::route_selected_step(
                    graph,
                    ranks,
                    &mut placed.layout.bounds,
                    channels,
                    true,
                    Some(&sides),
                )
                .await;
                placed.layout.width = routed.width;
                placed.layout.paths = routed.paths;
                placed.layout.junctions = routed.junctions;
            }
            Strategy::Nested => {
                let routed =
                    containers::routing::route_selected_step(graph, &placed.layout, &sides).await;
                placed.layout.paths = routed.paths;
                placed.layout.junctions = routed.junctions;
            }
        }
    }
    placed.layout.positions = placed
        .layout
        .bounds
        .iter()
        .map(|bounds| bounds.origin)
        .collect();
    debug_assert!(!super::routing_shared::slots::opposite_roles_collide(
        graph,
        &placed.layout.paths
    ));
}
