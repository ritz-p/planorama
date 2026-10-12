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
                let from = layout.bounds[edge.from];
                let to = layout.bounds[edge.to];
                for (point, bounds) in [(path[0], from), (*path.last().unwrap(), to)] {
                    assert!(
                        ((point.x == bounds.origin.x || point.x == bounds.right())
                            && (bounds.origin.y..=bounds.origin.y + bounds.height)
                                .contains(&point.y))
                            || ((point.y == bounds.origin.y
                                || point.y == bounds.origin.y + bounds.height)
                                && (bounds.origin.x..=bounds.right()).contains(&point.x))
                    );
                }
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
                        relationship_markers::terminals(
                            &graph,
                            0,
                            relationship_markers::numbers(&graph)[0],
                            [a, b],
                        ),
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

fn independent_associations(count: usize) -> Graph {
    let mut changes = Vec::new();
    let mut resources = Vec::new();
    for i in 0..count {
        let subnet = format!("aws_subnet.n{i:03}");
        let table = format!("aws_route_table.n{i:03}");
        let association = format!("aws_route_table_association.n{i:03}");
        for (address, kind) in [
            (&subnet, "aws_subnet"),
            (&table, "aws_route_table"),
            (&association, "aws_route_table_association"),
        ] {
            changes.push(serde_json::json!({"address": address, "type": kind, "change": {"actions": ["create"]}}));
        }
        resources.extend([
            serde_json::json!({"address": subnet}),
            serde_json::json!({"address": table}),
            serde_json::json!({"address": association, "expressions": {
                "subnet_id": {"references": [format!("{subnet}.id")]},
                "route_table_id": {"references": [format!("{table}.id")]}
            }}),
        ]);
    }
    crate::semantic::transform(
        &crate::plan::parse(
            &serde_json::json!({
                "format_version": "1.2", "resource_changes": changes,
                "configuration": {"root_module": {"resources": resources}}
            })
            .to_string(),
        )
        .unwrap(),
    )
    .0
}

#[test]
fn compact_single_digit_terminals_survive_selected_sides_and_final_passes() {
    use crate::layout::Point;
    use crate::layout::routing_shared::{refinement, spacing};
    let mut graph = independent_associations(12);
    let snapshot = |graph: &Graph, vertical: bool| {
        let mut layout = Layout::new(graph);
        let numbers = relationship_markers::numbers(graph);
        assert!(numbers.iter().any(|n| n[1] == Some(12)));
        for (index, edge) in graph.edges.iter().enumerate() {
            let number = numbers[index][1].unwrap();
            let from = Point {
                x: 100 + if vertical { (number - 1) * 600 } else { 0 },
                y: 100 + if vertical { 0 } else { (number - 1) * 400 },
            };
            let to = Point {
                x: from.x + if vertical { 0 } else { 284 },
                y: from.y + if vertical { 192 } else { 0 },
            };
            for (node, origin) in [(edge.from, from), (edge.to, to)] {
                layout.bounds[node] = Bounds {
                    origin,
                    width: 200,
                    height: 120,
                };
                layout.positions[node] = origin;
                layout.header_heights[node] = 120;
            }
        }
        layout.width = layout.bounds.iter().map(|b| b.right()).max().unwrap() + 160;
        layout.height = layout
            .bounds
            .iter()
            .map(|b| b.origin.y + b.height)
            .max()
            .unwrap()
            + 160;
        let routed = crate::layout::containers::routing::route_selected(
            graph,
            &layout,
            &vec![[Side::Right; 2]; graph.edges.len()],
        );
        layout.paths = routed.paths;
        layout.junctions = routed.junctions;
        refinement::refine(graph, &mut layout);
        spacing::refine(graph, &mut layout);
        let index = numbers.iter().position(|n| n[1] == Some(1)).unwrap();
        let path = &layout.paths[index];
        let shape = spacing::Shape::measure(path);
        assert_eq!(
            (shape.length, shape.bends, shape.reversals),
            (if vertical { 72 } else { 84 }, 0, 0),
            "{path:?}"
        );
        let pair = [refinement::terminal(path, 0), refinement::terminal(path, 1)];
        let clearances =
            relationship_markers::terminals(graph, index, numbers[index], pair.map(|p| p.side));
        assert!(ports::valid_with_clearances(
            path,
            pair[0],
            pair[1],
            &spacing::barriers(graph, &layout, index),
            clearances
        ));
        let mut result = std::collections::BTreeMap::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            result.insert(
                edge.change.as_ref().unwrap().address.clone(),
                layout.paths[index].clone(),
            );
        }
        result
    };
    for vertical in [false, true] {
        let original = snapshot(&graph, vertical);
        graph.edges.reverse();
        graph.relationships.reverse();
        assert_eq!(original, snapshot(&graph, vertical));
    }
}
