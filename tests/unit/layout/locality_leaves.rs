use super::*;
use crate::layout::{ContainmentTree, containers, metrics};
use crate::model::Edge;
use serde_json::json;

fn graph() -> Graph {
    let input = json!({"format_version":"1.2", "resource_changes": (0..7).map(|i|
        json!({"address":format!("test.n{i}"), "type":"test", "change":{"actions":["create"]}})
    ).collect::<Vec<_>>()});
    let mut graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
    graph.nodes[0].role = ResourceRole::Container;
    graph.edges = (1..7)
        .map(|to| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((0, to))
        })
        .collect();
    graph
        .edges
        .extend([(1, 2), (1, 3), (1, 4), (4, 5), (5, 1)].map(Edge::from));
    graph.edges.push(Edge {
        kind: EdgeKind::Association,
        ..Edge::from((1, 2))
    });
    graph
}

fn controlled(graph: &Graph) -> Layout<'_> {
    let mut layout = containers::place_geometry(graph, ContainmentTree::new(graph), true);
    layout.bounds[0] = Bounds {
        origin: Point { x: 60, y: 160 },
        width: 2200,
        height: 1100,
    };
    for (node, (x, y)) in [
        (200, 350),
        (1600, 850),
        (1600, 650),
        (700, 350),
        (1100, 350),
        (700, 700),
    ]
    .into_iter()
    .enumerate()
    {
        layout.bounds[node + 1].origin = Point { x, y };
    }
    layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    layout.width = 2340;
    layout.height = 1340;
    layout
}

fn gap(a: Bounds, b: Bounds) -> usize {
    a.origin
        .x
        .saturating_sub(b.right())
        .max(b.origin.x.saturating_sub(a.right()))
        + a.origin
            .y
            .saturating_sub(b.origin.y + b.height)
            .max(b.origin.y.saturating_sub(a.origin.y + a.height))
}

#[test]
fn parallel_edges_count_once_and_logical_projection_removes_phantom_peers() {
    let mut graph = graph();
    graph.edges.push(Edge::from((2, 2)));
    assert_eq!(leaves(&graph), [(2, 1), (3, 1)]);
    graph.nodes[6].placement = crate::model::Placement::Logical;
    graph.edges.push(Edge::from((6, 2)));
    assert!(!leaves(&graph).iter().any(|&(node, _)| node == 2));
    let projected = crate::view::spatial::project(&graph);
    let leaf = projected
        .graph
        .nodes
        .iter()
        .position(|n| n.address == "test.n2")
        .unwrap();
    assert!(
        leaves(&projected.graph)
            .iter()
            .any(|&(node, _)| node == leaf)
    );
    graph.nodes.truncate(3);
    graph.nodes[0].role = ResourceRole::Node;
    graph.edges = [(0, 1), (1, 2)].map(Edge::from).into();
    assert_eq!(leaves(&graph), [(0, 1), (2, 1)]);
    graph.nodes.truncate(2);
    graph.edges = vec![Edge::from((0, 1))];
    assert_eq!(leaves(&graph), [(1, 0)]);
}

#[test]
fn leaves_shorten_routes_without_moving_hub_or_expanding_area() {
    let graph = graph();
    let original = graph.clone();
    let mut old = controlled(&graph);
    let mut new = controlled(&graph);
    refine(&graph, &mut new);
    for node in [0, 1, 4, 5, 6] {
        assert_eq!(new.bounds[node], old.bounds[node]);
    }
    for leaf in [2, 3] {
        assert!(gap(new.bounds[leaf], new.bounds[1]) < gap(old.bounds[leaf], old.bounds[1]));
        for other in 1..7 {
            if leaf != other {
                assert!(separated(
                    new.bounds[leaf],
                    new.bounds[other],
                    super::super::super::relationship_markers::padding(&graph)
                ));
            }
        }
    }
    containers::route(&graph, &mut old, true);
    containers::route(&graph, &mut new, true);
    let before = metrics::measure(&old.paths);
    let after = metrics::measure(&new.paths);
    eprintln!(
        "leaf locality: gaps {:?} -> {:?}; routes {before:?} -> {after:?}; area {} -> {}",
        [
            gap(old.bounds[2], old.bounds[1]),
            gap(old.bounds[3], old.bounds[1])
        ],
        [
            gap(new.bounds[2], new.bounds[1]),
            gap(new.bounds[3], new.bounds[1])
        ],
        old.width * old.height,
        new.width * new.height
    );
    assert!(after.total_path_length < before.total_path_length);
    assert!(after.bend_count <= before.bend_count);
    assert_eq!(old.width * old.height, new.width * new.height);
    assert_eq!(graph, original);
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    let mut permuted = controlled(&reordered);
    refine(&reordered, &mut permuted);
    assert_eq!(new.bounds, permuted.bounds);
    let snapshot = |graph: &Graph| {
        let layout = Layout::new(graph);
        graph
            .nodes
            .iter()
            .zip(layout.bounds)
            .map(|(n, b)| (n.entity.id.clone(), b))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    reordered.nodes.reverse();
    let count = reordered.nodes.len();
    for edge in &mut reordered.edges {
        edge.from = count - 1 - edge.from;
        edge.to = count - 1 - edge.to;
    }
    assert_eq!(snapshot(&graph), snapshot(&reordered));
}

#[test]
fn cross_subnet_leaf_stays_inside_its_existing_parent() {
    let mut graph = graph();
    graph.nodes[1].role = ResourceRole::Container;
    graph.nodes[2].role = ResourceRole::Container;
    graph.edges = [(0, 1), (0, 2), (1, 3), (2, 4), (2, 5), (0, 6)]
        .map(|pair| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from(pair)
        })
        .into();
    graph.edges.extend([(3, 4), (4, 5)].map(Edge::from));
    let mut layout = controlled(&graph);
    layout.bounds[1] = Bounds {
        origin: Point { x: 100, y: 350 },
        width: 800,
        height: 700,
    };
    layout.bounds[2] = Bounds {
        origin: Point { x: 1100, y: 350 },
        width: 800,
        height: 700,
    };
    layout.bounds[3].origin = Point { x: 160, y: 550 };
    layout.bounds[4].origin = Point { x: 1500, y: 550 };
    layout.bounds[5].origin = Point { x: 1500, y: 850 };
    layout.bounds[6].origin = Point { x: 900, y: 1100 };
    let before = layout.bounds[3];
    let parents = layout.containment.parents.clone();
    refine(&graph, &mut layout);
    assert_eq!(parents, layout.containment.parents);
    assert!(layout.bounds[3].origin.x > before.origin.x);
    assert!(layout.bounds[3].right() + 40 <= layout.bounds[1].right());
    assert!(layout.bounds[3].origin.y >= layout.bounds[1].origin.y + layout.header_heights[1] + 40);
    assert!(
        layout.bounds[3].origin.y + layout.bounds[3].height + 40
            <= layout.bounds[1].origin.y + layout.bounds[1].height
    );
}
