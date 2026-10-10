use super::{Bounds, ContainmentTree, Layout, NODE_HEIGHT, containers, placement, rank, routing};
use crate::model::architecture::{Graph, ResourceRole};

enum Strategy {
    Flat {
        ranks: Vec<usize>,
        channels: Vec<usize>,
    },
    Nested,
}

#[test]
fn dense_horizontal_ports_expand_flat_rows_and_nested_containers() {
    use crate::model::{Action, Directionality, Edge, EdgeChange, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.c","type":"test"}]}"#).unwrap();
    for nested in [false, true] {
        let mut graph = crate::semantic::transform(&raw).0;
        graph.edges = (0..8)
            .map(|i| Edge {
                kind: EdgeKind::Association,
                change: Some(EdgeChange {
                    directionality: Directionality::Directed,
                    previous_address: None,
                    metadata: Default::default(),
                    address: format!("test.link{i}"),
                    action: Action::Create,
                }),
                ..Edge::from(if i < 4 { (0, 1) } else { (1, 2) })
            })
            .collect();
        if nested {
            let mut parent = graph.nodes[0].clone();
            parent.role = ResourceRole::Container;
            parent.address = "test.parent".into();
            graph.nodes.push(parent);
            graph.edges.extend((0..3).map(|child| Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((3, child))
            }));
        }
        let layout = Layout::new(&graph);
        assert_eq!(layout.bounds[1].height, 105, "nested={nested}");
        for indices in [0..4, 4..8] {
            let mut ys: Vec<_> = indices
                .map(|i| {
                    if i < 4 {
                        layout.paths[i].last().unwrap().y
                    } else {
                        layout.paths[i][0].y
                    }
                })
                .collect();
            ys.sort();
            assert!(ys.windows(2).all(|p| p[1] - p[0] >= 16));
        }
        if nested {
            for child in &layout.bounds[..3] {
                assert!(
                    child.origin.y + child.height
                        < layout.bounds[3].origin.y + layout.bounds[3].height
                );
            }
        }
        let expected = layout.bounds;
        graph.edges.reverse();
        let reversed = Layout::new(&graph);
        assert_eq!(expected, reversed.bounds);
    }
}
pub(super) struct Placed<'a> {
    pub layout: Layout<'a>,
    strategy: Strategy,
}

pub(super) fn place(graph: &Graph, tree: ContainmentTree) -> Placed<'_> {
    place_sized(graph, tree, None)
}

fn place_sized(graph: &Graph, tree: ContainmentTree, heights: Option<Vec<usize>>) -> Placed<'_> {
    if graph.nodes.iter().any(|node| {
        node.role == ResourceRole::Container
            || node
                .entity
                .scope
                .as_ref()
                .is_some_and(|s| s.region.is_some())
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
                vec![NODE_HEIGHT; graph.nodes.len()]
            }
        });
        let placement = placement::place_with_heights(graph, &ranks, &header_heights);
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

pub(super) fn route<'a>(graph: &'a Graph, placed: &mut Placed<'a>) {
    match &placed.strategy {
        Strategy::Flat { ranks, channels } => {
            let routed = routing::route(graph, ranks, &mut placed.layout.bounds, channels);
            placed.layout.width = routed.width;
            placed.layout.paths = routed.paths;
            placed.layout.junctions = routed.junctions;
        }
        Strategy::Nested => containers::route(graph, &mut placed.layout, true),
    }
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
    let sides: Vec<_> = placed
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
    let demand = super::routing_shared::slots::required_heights(graph, &sides, &visible);
    let heights: Vec<_> = demand
        .iter()
        .enumerate()
        .map(|(node, &height)| {
            let inferred = placed.layout.containment.parents[node]
                .is_some_and(|parent| graph.derived_containment(parent, node));
            height.max(NODE_HEIGHT + if inferred { 18 } else { 0 })
        })
        .collect();
    if heights != placed.layout.header_heights
        || super::routing_shared::slots::crowded(graph, &placed.layout.paths, &sides)
    {
        *placed = place_sized(graph, ContainmentTree::new(graph), Some(heights));
        match &placed.strategy {
            Strategy::Flat { ranks, channels } => {
                let routed = routing::route_selected(
                    graph,
                    ranks,
                    &mut placed.layout.bounds,
                    channels,
                    true,
                    Some(&sides),
                );
                placed.layout.width = routed.width;
                placed.layout.paths = routed.paths;
                placed.layout.junctions = routed.junctions;
            }
            Strategy::Nested => {
                let routed = containers::routing::route_selected(graph, &placed.layout, &sides);
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
}
