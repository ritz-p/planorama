use super::*;

#[test]
fn parallel_directed_numbered_edges_keep_distinct_endpoints() {
    use std::collections::BTreeMap;
    let mut graph = crate::semantic::transform(
        &crate::plan::parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/routes-plan.json"
        )))
        .unwrap(),
    )
    .0;
    let snapshot = |graph: &Graph| {
        let layout = Layout::new(graph);
        let mut endpoints = BTreeMap::new();
        let mut paths = BTreeMap::new();
        for (edge, path) in graph.edges.iter().zip(&layout.paths) {
            if path.is_empty() {
                continue;
            }
            if edge.change.is_some() {
                assert_eq!(path[0].x, layout.bounds[edge.from].right());
                assert_eq!(path.last().unwrap().x, layout.bounds[edge.to].origin.x);
            }
            for (node, point) in [(edge.from, path[0]), (edge.to, *path.last().unwrap())] {
                assert!(
                    endpoints.insert((node, point.x, point.y), edge).is_none(),
                    "duplicate endpoint for {} at {point:?}",
                    graph.nodes[node].address
                );
            }
            paths.insert(
                (
                    edge.from,
                    edge.to,
                    edge.change.as_ref().map(|c| c.address.clone()),
                ),
                path.clone(),
            );
        }
        paths
    };
    let original = snapshot(&graph);
    graph.edges.reverse();
    assert_eq!(original, snapshot(&graph));
}

#[test]
fn packed_siblings_leave_room_for_both_numbered_terminals() {
    use crate::model::{Edge, EdgeKind, ResourceRole};
    let base = crate::semantic::transform(
        &crate::plan::parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/association-plan.json"
        )))
        .unwrap(),
    )
    .0;
    for nested in [false, true] {
        let mut graph = base.clone();
        if nested {
            let parent = graph.nodes.len();
            let mut node = graph.nodes[0].clone();
            node.address = "aws_vpc.parent".into();
            node.role = ResourceRole::Container;
            graph.nodes.push(node);
            for child in 0..parent {
                graph.edges.push(Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((parent, child))
                });
            }
        }
        let layout = Layout::new(&graph);
        let edge = &graph.edges[0];
        let path = &layout.paths[0];
        let (from, to) = (layout.bounds[edge.from], layout.bounds[edge.to]);
        let pairs = ports::facing_pairs(from, to);
        assert!(
            pairs.iter().any(|&(a, b)| {
                let start = from.port(
                    a,
                    if matches!(a, Side::Left | Side::Right) {
                        path[0].y.saturating_sub(from.origin.y)
                    } else {
                        path[0].x.saturating_sub(from.origin.x)
                    },
                );
                let last = *path.last().unwrap();
                let end = to.port(
                    b,
                    if matches!(b, Side::Left | Side::Right) {
                        last.y.saturating_sub(to.origin.y)
                    } else {
                        last.x.saturating_sub(to.origin.x)
                    },
                );
                start.point == path[0]
                    && end.point == last
                    && ports::valid_with_clearances(
                        path,
                        start,
                        end,
                        &obstacles(&layout, edge),
                        [relationship_markers::clearance(&graph); 2],
                    )
            }),
            "nested={nested}, bounds={from:?}, {to:?}, path={path:?}"
        );
    }
}

#[test]
fn numbered_relationships_choose_facing_source_and_target_sides() {
    use crate::layout::Point;
    let graph = crate::semantic::transform(
        &crate::plan::parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/association-plan.json"
        )))
        .unwrap(),
    )
    .0;
    for (positions, sides) in [
        ([(100, 100), (600, 100)], [Side::Right, Side::Left]),
        ([(600, 100), (100, 100)], [Side::Left, Side::Right]),
        ([(100, 100), (100, 600)], [Side::Bottom, Side::Top]),
    ] {
        let mut layout = Layout::new(&graph);
        let bounds: Vec<_> = positions
            .map(|(x, y)| Bounds {
                origin: Point { x, y },
                width: 200,
                height: 120,
            })
            .into();
        layout.bounds[graph.edges[0].from] = bounds[0];
        layout.bounds[graph.edges[0].to] = bounds[1];
        crate::layout::routing_shared::search::GRID_SEARCH_COUNT.set(0);
        let terminals = Terminals::new(
            &graph,
            &layout,
            &[0],
            (&[60], &[60]),
            (&[[60, 60, 100, 100]], &[[60, 60, 100, 100]]),
            true,
        );
        assert_eq!(terminals.ports[0].map(|p| p.side), sides);
        assert_eq!(
            crate::layout::routing_shared::search::GRID_SEARCH_COUNT.get(),
            0
        );
        let [start, end] = terminals.ports[0];
        let barriers = terminals.barriers(&layout, &graph, 0);
        let path =
            ports::connect_with_clearances(start, end, &barriers, None, terminals.clearances[0])
                .unwrap();
        assert!(ports::valid_with_clearances(
            &path,
            start,
            end,
            &barriers,
            terminals.clearances[0]
        ));
    }
}
