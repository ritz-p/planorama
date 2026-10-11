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
