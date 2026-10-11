use super::*;

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
