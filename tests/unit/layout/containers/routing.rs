use super::*;

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
