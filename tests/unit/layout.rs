use super::*;
use crate::model::{Action, Node};

fn graph(count: usize, edges: Vec<(usize, usize)>) -> Graph {
    Graph {
        relationships: Vec::new(),
        components: Vec::new(),
        checks: Vec::new(),
        status: Default::default(),
        nodes: (0..count)
            .map(|i| Node {
                entity: crate::model::ArchitectureEntity::terraform(
                    crate::model::TerraformEntityId {
                        address: format!("test.n{i}"),
                        deposed_key: None,
                    },
                ),
                deposed_key: None,
                previous_address: None,
                metadata: Default::default(),
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                provider: crate::model::ProviderIdentity::inferred("test"),
                provider_configuration: None,
                module: "root".into(),
                action: Action::Create,
                mode: crate::model::EntityMode::Managed,
                role: crate::model::ResourceRole::Unknown,
            })
            .collect(),
        edges: edges.into_iter().map(crate::model::Edge::from).collect(),
    }
}

fn check_geometry(graph: &Graph) {
    let layout = Layout::new(graph);
    for (i, a) in layout.positions.iter().enumerate() {
        assert!(a.x + NODE_WIDTH <= layout.width && a.y + NODE_HEIGHT <= layout.height);
        for b in layout.positions.iter().skip(i + 1) {
            assert!(
                a.x + NODE_WIDTH <= b.x
                    || b.x + NODE_WIDTH <= a.x
                    || a.y + NODE_HEIGHT <= b.y
                    || b.y + NODE_HEIGHT <= a.y
            );
        }
    }
    for path in &layout.paths {
        for segment in path.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            assert!(a.x == b.x || a.y == b.y);
            assert!(a.x <= layout.width && a.y <= layout.height);
            for node in &layout.positions {
                let crosses = if a.x == b.x {
                    a.x > node.x
                        && a.x < node.x + NODE_WIDTH
                        && a.y.max(b.y) > node.y
                        && a.y.min(b.y) < node.y + NODE_HEIGHT
                } else {
                    a.y > node.y
                        && a.y < node.y + NODE_HEIGHT
                        && a.x.max(b.x) > node.x
                        && a.x.min(b.x) < node.x + NODE_WIDTH
                };
                assert!(!crosses, "edge {segment:?} crosses node {node:?}");
            }
        }
    }
}

#[test]
fn sweeps_uncross_reversed_dependencies() {
    let graph = graph(4, vec![(0, 3), (1, 2)]);
    let positions = Layout::new(&graph).positions;
    assert_eq!(
        positions[0].y < positions[1].y,
        positions[3].y < positions[2].y
    );
}

#[test]
fn bundled_stars_avoid_cards_and_keep_individual_svg_relationships() {
    for edges in [vec![(0, 1), (0, 2), (0, 3)], vec![(0, 3), (1, 3), (2, 3)]] {
        let graph = graph(4, edges);
        check_geometry(&graph);
        let layout = Layout::new(&graph);
        let svg = crate::svg::render(&graph, &layout);
        assert_eq!(svg.matches("marker-end=").count(), 3);
        assert!(!layout.junctions.is_empty());
        assert_eq!(svg.matches("<circle ").count(), layout.junctions.len());
        for edge in &graph.edges {
            let (a, b) = edge.endpoints();
            assert!(svg.contains(&format!("<title>test.n{a} → test.n{b}</title>")));
        }
        assert_eq!(svg, crate::svg::render(&graph, &Layout::new(&graph)));
    }
}

#[test]
fn bundling_sample_preserves_both_stages_of_dependencies() {
    let graph = crate::semantic::base_graph(
        &crate::plan::parse(include_str!("../../examples/bundling-plan.json")).unwrap(),
    );
    assert_eq!(graph.nodes.len(), 5);
    assert_eq!(graph.edges.len(), 6);
    let layout = Layout::new(&graph);
    assert_eq!(layout.paths.len(), 6);
    assert!(!layout.junctions.is_empty());
    check_geometry(&graph);
}

#[test]
fn virtual_nodes_do_not_become_resources_or_visible_edges() {
    let graph = graph(4, vec![(0, 1), (1, 2), (2, 3), (0, 3)]);
    let original_addresses: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| node.address.clone())
        .collect();
    let layout = Layout::new(&graph);
    assert_eq!(layout.positions.len(), graph.nodes.len());
    assert_eq!(layout.paths.len(), graph.edges.len());
    let svg = crate::svg::render(&graph, &layout);
    assert!(svg.contains("4 resources, 4 reference edges"));
    assert_eq!(svg.matches("marker-end=").count(), 4);
    assert_eq!(
        original_addresses,
        graph
            .nodes
            .iter()
            .map(|node| node.address.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(svg, crate::svg::render(&graph, &Layout::new(&graph)));
    check_geometry(&graph);
}

#[test]
fn long_edges_and_cycles_avoid_cards() {
    check_geometry(&graph(
        6,
        vec![(0, 1), (1, 2), (2, 3), (0, 3), (3, 4), (4, 3), (0, 5)],
    ));
    check_geometry(&crate::semantic::base_graph(
        &crate::plan::parse(include_str!("../../examples/plan.json")).unwrap(),
    ));
}

#[test]
fn dense_dag_and_multiple_modules_avoid_cards() {
    let mut edges = Vec::new();
    for a in 0..15 {
        for b in a + 1..15 {
            if (a + b) % 3 != 0 {
                edges.push((a, b));
            }
        }
    }
    let mut graph = graph(15, edges);
    for (i, node) in graph.nodes.iter_mut().enumerate() {
        node.module = format!("module.group{}", i % 3);
    }
    check_geometry(&graph);
    check_geometry(&crate::semantic::base_graph(
        &crate::plan::parse(include_str!("../../tests/fixtures/terraform-plan.json")).unwrap(),
    ));
}

#[test]
fn dense_graph_quality_metrics_are_deterministic() {
    let edges = (0..4)
        .flat_map(|a| (4..12).map(move |b| (a, b)))
        .chain((4..8).flat_map(|a| (8..12).map(move |b| (a, b))))
        .collect();
    let graph = graph(12, edges);
    check_geometry(&graph);
    let metrics = metrics::measure(&Layout::new(&graph).paths);
    assert_eq!(
        metrics,
        metrics::LayoutMetrics {
            overlap_distance: 2551,
            crossing_count: 319,
            bend_count: 160,
            total_path_length: 73616,
        }
    );
    assert_eq!(metrics, metrics::measure(&Layout::new(&graph).paths));
}

#[test]
fn dense_bipartite_graph_expands_gutters_without_crossing_cards() {
    let graph = graph(
        16,
        (0..8).flat_map(|a| (8..16).map(move |b| (a, b))).collect(),
    );
    check_geometry(&graph);
    let layout = Layout::new(&graph);
    assert!(layout.positions[8].x - layout.positions[0].x > 420);
    assert_eq!(layout.paths, Layout::new(&graph).paths);
}
