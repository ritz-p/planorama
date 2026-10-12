use super::*;

#[test]
fn numbered_channels_are_kept_compact_beside_flexible_plain_routes() {
    use crate::layout::relationship_markers;
    use crate::layout::routing_shared::{ports, refinement, spacing};
    use crate::model::{Directionality, RelationshipProvenance};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.y","type":"test"},{"address":"test.z","type":"test"}]}"#).unwrap();
    let numbered = crate::semantic::transform(
        &crate::plan::parse(include_str!("../../../fixtures/association-plan.json")).unwrap(),
    )
    .0;
    for (kind, direction, gap) in [
        (EdgeKind::Association, Directionality::Directed, 84),
        (EdgeKind::Association, Directionality::Undirected, 84),
        (EdgeKind::Dependency, Directionality::Directed, 58),
    ] {
        let mut graph = crate::semantic::transform(&raw).0;
        let mut edge = numbered.edges[0].clone();
        edge.from = 2;
        edge.to = 3;
        edge.change.as_mut().unwrap().directionality = direction;
        let mut relationship = numbered
            .relationships
            .iter()
            .find(|r| r.kind == edge.kind)
            .unwrap()
            .clone();
        edge.kind = kind;
        relationship.kind = kind;
        relationship.from = graph.nodes[2].entity.id.clone();
        relationship.to = graph.nodes[3].entity.id.clone();
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { change, .. } = provenance {
                change.directionality = direction;
            }
        }
        graph.edges = vec![Edge::from((0, 1)), edge];
        graph.relationships = vec![relationship];
        let snapshot = |graph: &Graph| {
            let bounds: Vec<_> = [
                (100, 350, 1000 - (84 - gap), 100),
                (1100 + gap, 400, 1000, 100),
                (100, 600, 1000, 200),
                (1100 + gap, 100, 1000, 200),
            ]
            .into_iter()
            .map(|(x, y, width, height)| Bounds {
                origin: Point { x, y },
                width,
                height,
            })
            .collect();
            let mut layout = Layout {
                scopes: vec![],
                header_heights: bounds.iter().map(|b| b.height).collect(),
                positions: bounds.iter().map(|b| b.origin).collect(),
                bounds,
                containers: vec![],
                containment: crate::layout::ContainmentTree::new(graph),
                width: 2400,
                height: 1000,
                bands: vec![],
                paths: vec![],
                junctions: vec![],
            };
            let routed = route_selected(graph, &layout, &[[Side::Right, Side::Left]; 2]);
            layout.paths = routed.paths;
            layout.junctions = routed.junctions;
            refinement::refine(graph, &mut layout);
            spacing::refine(graph, &mut layout);
            assert_eq!(
                crate::layout::metrics::measure(&layout.paths).overlap_distance,
                0
            );
            let numbers = relationship_markers::numbers(graph);
            let mut result = std::collections::BTreeMap::new();
            for (index, edge) in graph.edges.iter().enumerate() {
                let path = &layout.paths[index];
                let shape = spacing::Shape::measure(path);
                assert_eq!((shape.reversals, shape.backtracking), (0, 0), "{path:?}");
                assert!(shape.bends <= 2, "{path:?}");
                let pair = [refinement::terminal(path, 0), refinement::terminal(path, 1)];
                let terminals = relationship_markers::terminals(
                    graph,
                    index,
                    numbers[index],
                    pair.map(|p| p.side),
                );
                assert!(ports::valid_with_clearances(
                    path,
                    pair[0],
                    pair[1],
                    &spacing::barriers(graph, &layout, index),
                    terminals,
                ));
                result.insert((edge.from, edge.to), path.clone());
            }
            result
        };
        let original = snapshot(&graph);
        graph.edges.reverse();
        graph.relationships.reverse();
        assert_eq!(original, snapshot(&graph));
    }
}

#[test]
fn foreign_container_shortcut_uses_soft_cost_and_keeps_endpoints_hard() {
    use crate::layout::routing_shared::{crosses, ports};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.c","type":"test"},{"address":"test.d","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.nodes[1].role = crate::model::ResourceRole::Container;
    graph.edges = vec![
        Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((1, 2))
        },
        Edge::from((0, 3)),
    ];
    let mut layout = Layout::new(&graph);
    layout.bounds[1] = Bounds {
        origin: Point { x: 400, y: 100 },
        width: 400,
        height: 600,
    };
    layout.header_heights[1] = 40;
    layout.bounds[2] = Bounds {
        origin: Point { x: 450, y: 300 },
        width: 300,
        height: 100,
    };
    layout.bounds[0] = Bounds {
        origin: Point { x: 100, y: 300 },
        width: 100,
        height: 100,
    };
    layout.bounds[3] = Bounds {
        origin: Point { x: 1000, y: 300 },
        width: 100,
        height: 100,
    };
    let edge = &graph.edges[1];
    let barriers = obstacles(&layout, edge);
    assert!(!barriers.contains(&layout.bounds[1].header(40)));
    assert!(barriers.contains(&layout.bounds[0]) && barriers.contains(&layout.bounds[3]));
    let mut scorer = Scorer::default();
    scorer.set_soft(soft_obstacles(&layout, edge));
    assert_eq!(scorer.soft()[0].bounds, layout.bounds[1]);
    assert_eq!(scorer.soft().len(), 1);
    assert_eq!(scorer.soft()[0].cost, 1);
    let start = layout.bounds[0].port(Side::Right, 50);
    let end = layout.bounds[3].port(Side::Left, 50);
    let path = ports::connect(start, end, &barriers, Some(&scorer)).unwrap();
    let mut old_barriers = barriers.clone();
    old_barriers.push(layout.bounds[1]);
    let old = ports::connect(start, end, &old_barriers, Some(&scorer)).unwrap();
    let length = |p: Vec<Point>| crate::layout::metrics::measure(&[p]).total_path_length;
    assert!(length(path.clone()) < length(old));
    assert!(
        path.windows(2)
            .any(|p| crosses(p[0], p[1], layout.bounds[1]))
    );
    assert!(
        path.windows(2)
            .all(|p| barriers.iter().all(|&b| !crosses(p[0], p[1], b)))
    );
    layout.bounds[1].height = 260;
    layout.bounds[1].width = 1000;
    layout.bounds[3].origin.x = 2000;
    let end = layout.bounds[3].port(Side::Left, 50);
    let mut scorer = Scorer::default();
    scorer.set_soft(soft_obstacles(&layout, edge));
    let detour = ports::connect(start, end, &obstacles(&layout, edge), Some(&scorer)).unwrap();
    assert!(
        detour
            .windows(2)
            .all(|p| !crosses(p[0], p[1], layout.bounds[1]))
    );
}

#[test]
fn external_container_endpoints_are_protected_but_descendants_remain_reachable() {
    use crate::layout::routing_shared::{crosses, ports};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.parent","type":"test"},{"address":"test.child","type":"test"},{"address":"test.external","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.nodes[0].role = crate::model::ResourceRole::Container;
    graph.edges = vec![Edge {
        kind: EdgeKind::Containment,
        ..Edge::from((0, 1))
    }];
    for (from, to) in [(0, 2), (2, 0), (0, 1), (1, 0)] {
        graph.edges.push(Edge {
            kind: EdgeKind::Connection,
            ..Edge::from((from, to))
        });
    }
    let layout = Layout::new(&graph);
    let bounds = layout.bounds[0];
    for index in [1, 2] {
        let barriers = obstacles(&layout, &graph.edges[index]);
        assert!(barriers.contains(&bounds));
        assert!(
            layout.paths[index]
                .windows(2)
                .all(|p| !crosses(p[0], p[1], bounds))
        );
    }
    for index in [3, 4] {
        assert!(!obstacles(&layout, &graph.edges[index]).contains(&bounds));
        assert!(!layout.paths[index].is_empty());
    }
    let start = bounds.port(Side::Right, bounds.height / 2);
    let end = Port {
        point: Point {
            x: bounds.right() + 120,
            y: start.point.y + 100,
        },
        side: Side::Left,
    };
    let reentry = [
        start.point,
        start.outward(16),
        Point {
            x: start.point.x + 16,
            y: start.point.y + 30,
        },
        Point {
            x: start.point.x - 16,
            y: start.point.y + 30,
        },
        Point {
            x: start.point.x - 16,
            y: end.point.y,
        },
        end.point,
    ];
    assert!(!ports::valid(
        &reentry,
        start,
        end,
        &obstacles(&layout, &graph.edges[1])
    ));
    let mut reversed = graph.clone();
    reversed.edges.reverse();
    let reordered = Layout::new(&reversed);
    assert_eq!(
        layout.paths,
        reordered.paths.into_iter().rev().collect::<Vec<_>>()
    );
}

#[test]
fn shortcut_pass_preserves_nested_ports_and_bundles() {
    let raw = crate::plan::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/spanning-dense-plan.json"
    )))
    .unwrap();
    let graph = crate::semantic::transform(&raw).0;
    let tree = crate::layout::ContainmentTree::new(&graph);
    let keys = tree.keys.clone();
    let layout = super::super::place_geometry(&graph, tree, true);
    let before = route_impl(&graph, &layout, &keys, true, true, false);
    let after = route_impl(&graph, &layout, &keys, true, true, true);
    for (a, b) in before.paths.iter().zip(&after.paths) {
        assert_eq!(a.first(), b.first());
        assert_eq!(a.last(), b.last());
        assert!(b.len() <= a.len());
    }
    assert_eq!(before.junctions, after.junctions);
    for (edge, (a, b)) in graph
        .edges
        .iter()
        .zip(before.paths.iter().zip(&after.paths))
    {
        if edge.kind == EdgeKind::Connection {
            assert_eq!(a, b);
        }
    }
    let a = crate::layout::metrics::measure(&before.paths);
    let b = crate::layout::metrics::measure(&after.paths);
    assert!(b.total_path_length <= a.total_path_length);
    assert!(b.overlap_distance <= a.overlap_distance);
    assert!(b.crossing_count <= a.crossing_count);
}

#[test]
fn refined_ancestor_edges_keep_allocated_sides_and_distinct_ports() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.parent","type":"test"},{"address":"test.child","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.nodes[0].role = crate::model::ResourceRole::Container;
    graph.edges = vec![Edge {
        kind: EdgeKind::Containment,
        ..Edge::from((0, 1))
    }];
    graph.edges.extend((0..3).map(|i| Edge {
        kind: EdgeKind::Association,
        change: Some(crate::model::EdgeChange {
            address: format!("test.link{i}"),
            directionality: crate::model::Directionality::Directed,
            action: crate::model::Action::Create,
            previous_address: None,
            metadata: Default::default(),
        }),
        ..Edge::from((0, 1))
    }));
    let layout =
        super::super::place_geometry(&graph, crate::layout::ContainmentTree::new(&graph), true);
    let routed = route_selected(&graph, &layout, &[[Side::Right; 2]; 4]);
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for path in &routed.paths[1..] {
        let start = path[0];
        let end = *path.last().unwrap();
        assert_eq!(start.x, layout.bounds[0].right());
        assert_eq!(end.x, layout.bounds[1].right());
        sources.push(start.y);
        targets.push(end.y);
    }
    for mut offsets in [sources, targets] {
        offsets.sort_unstable();
        assert!(
            offsets
                .windows(2)
                .all(|p| p[1] - p[0] >= crate::layout::routing_shared::slots::MIN_SPACING)
        );
    }
}
