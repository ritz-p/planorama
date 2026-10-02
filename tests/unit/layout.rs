use super::*;
use crate::model::{Action, Node};

fn graph(count: usize, edges: Vec<(usize, usize)>) -> Graph {
    Graph {
        nodes: (0..count)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
            })
            .collect(),
        edges,
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
fn long_edges_and_cycles_avoid_cards() {
    check_geometry(&graph(
        6,
        vec![(0, 1), (1, 2), (2, 3), (0, 3), (3, 4), (4, 3), (0, 5)],
    ));
    check_geometry(&crate::plan::parse(include_str!("../../examples/plan.json")).unwrap());
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
    check_geometry(
        &crate::plan::parse(include_str!("../../tests/fixtures/terraform-plan.json")).unwrap(),
    );
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
