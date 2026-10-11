use super::*;
use crate::layout::Point;
use crate::model::Edge;

#[test]
fn changed_port_sides_reroute_even_when_card_heights_already_fit() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = (0..24)
        .map(|i| Edge::from(if i % 2 == 0 { (0, 1) } else { (1, 0) }))
        .collect();
    let mut placed = place_sized(&graph, ContainmentTree::new(&graph), Some(vec![425; 2]));
    placed.layout.bounds[0].origin = Point { x: 100, y: 100 };
    placed.layout.bounds[1].origin = Point { x: 800, y: 800 };
    placed.layout.paths = graph
        .edges
        .iter()
        .enumerate()
        .map(|(i, edge)| {
            let port = |node: usize| Point {
                x: placed.layout.bounds[node].origin.x + 24 + i * 10,
                y: placed.layout.bounds[node].origin.y,
            };
            let start = port(edge.from);
            let end = port(edge.to);
            vec![
                start,
                Point {
                    y: start.y - 30,
                    ..start
                },
                Point {
                    x: end.x,
                    y: start.y - 30,
                },
                Point {
                    y: end.y - 30,
                    ..end
                },
                end,
            ]
        })
        .collect();
    refine(&graph, &mut placed);
    assert_eq!(placed.layout.header_heights, vec![425; 2]);
    for (edge, path) in graph.edges.iter().zip(&placed.layout.paths) {
        for (node, point) in [
            (edge.from, path.first().unwrap()),
            (edge.to, path.last().unwrap()),
        ] {
            let bounds = placed.layout.bounds[node];
            assert_eq!(
                point.x,
                if node == 0 {
                    bounds.right()
                } else {
                    bounds.origin.x
                }
            );
            assert!(point.y > bounds.origin.y);
        }
    }
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
        let expected_paths = layout.paths;
        graph.edges.reverse();
        let reversed = Layout::new(&graph);
        assert_eq!(expected, reversed.bounds);
        assert_eq!(
            expected_paths,
            reversed.paths.into_iter().rev().collect::<Vec<_>>()
        );
    }
}
