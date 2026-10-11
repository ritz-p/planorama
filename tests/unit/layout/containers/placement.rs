use super::*;

#[test]
fn dense_hub_packing_reduces_final_routing_cost() {
    let mut graph = ranked_fixture();
    let template = graph.nodes[1].clone();
    graph.nodes.truncate(1);
    for node in 1..=27 {
        let mut item = template.clone();
        item.address = format!("test.node{node:02}");
        item.entity =
            crate::model::ArchitectureEntity::terraform(crate::model::TerraformEntityId {
                address: item.address.clone(),
                deposed_key: None,
            });
        graph.nodes.push(item);
    }
    graph.edges = (1..=27)
        .map(|node| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((0, node))
        })
        .collect();
    graph
        .edges
        .extend((2..=27).flat_map(|node| [Edge::from((1, node)), Edge::from((node, 1))]));
    let tree = crate::layout::ContainmentTree::new(&graph);
    let heights: Vec<_> = (0..28).map(|node| 100 + node % 5 * 30).collect();
    let mut improved = place_with_heights(&graph, tree.clone(), true, heights.clone());
    let mut baseline = place_with_heights(&graph, tree.clone(), true, heights);
    let columns = placement::columns(&graph, Some(0), &tree.children[0], &tree.parents);
    let sizes: Vec<_> = baseline
        .bounds
        .iter()
        .map(|b| (b.width, b.height))
        .collect();
    let mut offsets = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
    let (width, height) = placement::pack(&columns, &[], &sizes, &mut offsets, PADDING);
    let relationships = placement::relationships(&graph, &tree.children[0], &tree.parents);
    placement::refine(
        &tree.children[0],
        &relationships,
        &sizes,
        &mut offsets,
        height,
        PADDING,
    );
    baseline.bounds[0].width = width + PADDING * 2;
    baseline.bounds[0].height = height + baseline.header_heights[0] + PADDING * 2;
    for &node in &tree.children[0] {
        baseline.positions[node] = Point {
            x: baseline.positions[0].x + PADDING + offsets[node].x,
            y: baseline.positions[0].y + baseline.header_heights[0] + PADDING + offsets[node].y,
        };
        baseline.bounds[node].origin = baseline.positions[node];
    }
    route(&graph, &mut baseline, false);
    route(&graph, &mut improved, false);
    let old = crate::layout::metrics::measure(&baseline.paths);
    let new = crate::layout::metrics::measure(&improved.paths);
    eprintln!("compact {old:?}; relationship {new:?}");
    assert!(new.total_path_length < old.total_path_length);
    assert!(new.crossing_count < old.crossing_count);
    verify_layout(&graph, &improved);
}

#[test]
fn nested_routing_improves_over_lexical_packing() {
    let mut graph = ranked_fixture();
    graph
        .edges
        .retain(|edge| edge.kind == EdgeKind::Containment);
    graph
        .edges
        .extend([(1, 4), (2, 3), (3, 6), (4, 5)].map(Edge::from));
    let tree = crate::layout::ContainmentTree::new(&graph);
    let mut baseline = place_geometry(&graph, tree.clone(), true);
    let columns = placement::columns(&graph, Some(0), &tree.children[0], &tree.parents);
    let sizes: Vec<_> = baseline
        .bounds
        .iter()
        .map(|b| (b.width, b.height))
        .collect();
    let mut offsets = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
    placement::pack(&columns, &[], &sizes, &mut offsets, PADDING);
    for &node in &tree.children[0] {
        baseline.positions[node] = Point {
            x: baseline.positions[0].x + PADDING + offsets[node].x,
            y: baseline.positions[0].y + baseline.header_heights[0] + PADDING + offsets[node].y,
        };
        baseline.bounds[node].origin = baseline.positions[node];
    }
    route(&graph, &mut baseline, true);
    let mut improved = place_geometry(&graph, tree, true);
    route(&graph, &mut improved, true);
    let old = crate::layout::metrics::measure(&baseline.paths);
    let new = crate::layout::metrics::measure(&improved.paths);
    eprintln!("lexical {old:?}; barycenter {new:?}");
    assert!(new.total_path_length < old.total_path_length);
    assert!(new.crossing_count <= old.crossing_count);
    verify_layout(&graph, &improved);
}

#[test]
fn barycenters_uncross_siblings_without_changing_columns() {
    let sizes = vec![(100, 80); 4];
    let mut offsets = vec![
        Point { x: 0, y: 0 },
        Point { x: 0, y: 120 },
        Point { x: 140, y: 0 },
        Point { x: 140, y: 120 },
    ];
    placement::refine(
        &[0, 1, 2, 3],
        &[(0, 3), (1, 2)],
        &sizes,
        &mut offsets,
        200,
        40,
    );
    assert_eq!(offsets[0].y, offsets[3].y);
    assert_eq!(offsets[1].y, offsets[2].y);
    assert_eq!(
        offsets.iter().map(|p| p.x).collect::<Vec<_>>(),
        vec![0, 0, 140, 140]
    );
}

#[test]
fn fan_in_and_fan_out_align_hubs_with_peer_centers() {
    for reverse in [false, true] {
        let sizes = vec![(100, 80); 4];
        let mut offsets = vec![
            Point { x: 0, y: 0 },
            Point { x: 0, y: 120 },
            Point { x: 0, y: 240 },
            Point { x: 140, y: 0 },
        ];
        let edges: Vec<_> = (0..3)
            .map(|node| if reverse { (3, node) } else { (node, 3) })
            .collect();
        placement::refine(&[0, 1, 2, 3], &edges, &sizes, &mut offsets, 320, 40);
        assert_eq!(offsets[3].y, 120);
    }
}

#[test]
fn symmetric_and_unconnected_peers_keep_their_order() {
    let sizes = vec![(100, 80); 4];
    let mut offsets = vec![
        Point { x: 0, y: 0 },
        Point { x: 0, y: 120 },
        Point { x: 140, y: 60 },
        Point { x: 280, y: 0 },
    ];
    let original = offsets.clone();
    placement::refine(
        &[0, 1, 2, 3],
        &[(0, 2), (1, 2)],
        &sizes,
        &mut offsets,
        200,
        40,
    );
    assert_eq!(offsets, original);
}

#[test]
fn descendant_and_external_relationships_project_to_scope_owners() {
    let mut graph = ranked_fixture();
    graph.nodes[1].role = ResourceRole::Container;
    graph.nodes[2].role = ResourceRole::Container;
    graph.edges = [(0, 1), (0, 2), (1, 3), (2, 4)]
        .map(|pair| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from(pair)
        })
        .into();
    graph
        .edges
        .extend([(3, 4), (3, 2), (5, 4), (3, 1)].map(Edge::from));
    let tree = crate::layout::ContainmentTree::new(&graph);
    assert_eq!(
        placement::relationships(&graph, &[1, 2], &tree.parents),
        vec![(1, 2), (1, 2)]
    );
    assert_eq!(
        placement::relationships(&graph, &tree.roots, &tree.parents),
        vec![(0, 5)]
    );
    let before = Layout::new(&graph);
    verify_layout(&graph, &before);
    let count = graph.nodes.len();
    let mut reordered = graph.clone();
    reordered.nodes.reverse();
    reordered.edges.reverse();
    for edge in &mut reordered.edges {
        edge.from = count - 1 - edge.from;
        edge.to = count - 1 - edge.to;
    }
    let after = Layout::new(&reordered);
    assert_eq!(
        before.bounds,
        after.bounds.iter().rev().copied().collect::<Vec<_>>()
    );
    assert_eq!(
        before.paths,
        after.paths.iter().rev().cloned().collect::<Vec<_>>()
    );
}
