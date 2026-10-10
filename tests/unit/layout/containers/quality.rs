use super::*;
use crate::layout::metrics::measure;

#[test]
fn container_port_stubs_are_not_immediately_retraced() {
    let raw = plan::parse(include_str!(
        "../../../../examples/terraform-large/plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&raw).0;
    let layout = Layout::new(&graph);
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        for points in path.windows(3) {
            let [a, b, c] = [points[0], points[1], points[2]];
            let reverses = (a.y == b.y
                && b.y == c.y
                && a.x.cmp(&b.x) != b.x.cmp(&c.x)
                && a.x != b.x
                && b.x != c.x)
                || (a.x == b.x
                    && b.x == c.x
                    && a.y.cmp(&b.y) != b.y.cmp(&c.y)
                    && a.y != b.y
                    && b.y != c.y);
            assert!(
                !reverses,
                "{} -> {} retraces {points:?}",
                graph.nodes[edge.from].address, graph.nodes[edge.to].address
            );
        }
    }
}

#[test]
fn symmetric_peers_keep_ports_and_routes_when_edges_are_reordered() {
    let mut graph = ranked_fixture();
    graph.nodes.truncate(4);
    for (node, module) in [(1, "a"), (2, "b")] {
        graph.nodes[node].address = format!("module.{module}.test.same");
        graph.nodes[node].module = format!("module.{module}");
    }
    graph.edges = (1..4)
        .map(|node| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((0, node))
        })
        .collect();
    graph.edges.extend([Edge::from((1, 3)), Edge::from((2, 3))]);
    let keys = ordering::structural_keys(&graph);
    assert_eq!(keys[1], keys[2]);
    let before = Layout::new(&graph);
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    let after = Layout::new(&reordered);
    assert_eq!(before.bounds, after.bounds);
    assert_eq!(
        before.paths,
        after.paths.into_iter().rev().collect::<Vec<_>>()
    );

    for nested in [true, false] {
        let mut original = graph.clone();
        if !nested {
            original
                .edges
                .retain(|edge| edge.kind != EdgeKind::Containment);
        }
        let baseline = Layout::new(&original);
        let mut reordered = original.clone();
        reordered.nodes.swap(1, 2);
        let remap = |node| match node {
            1 => 2,
            2 => 1,
            other => other,
        };
        for edge in &mut reordered.edges {
            edge.from = remap(edge.from);
            edge.to = remap(edge.to);
        }
        reordered.edges.reverse();
        let result = Layout::new(&reordered);
        for node in 0..original.nodes.len() {
            assert_eq!(baseline.bounds[node], result.bounds[remap(node)]);
        }
        assert_eq!(
            baseline.paths,
            result.paths.into_iter().rev().collect::<Vec<_>>()
        );
    }
}

#[test]
fn coincident_crossings_make_a_long_clear_detour_worthwhile() {
    use crate::layout::routing_shared::scoring::Scorer;
    let start = Point { x: 100, y: 2000 };
    let end = Point { x: 500, y: 2000 };
    let obstacles = [
        Bounds {
            origin: Point { x: 200, y: 2440 },
            width: 100,
            height: 100,
        },
        Bounds {
            origin: Point { x: 600, y: 2040 },
            width: 100,
            height: 100,
        },
    ];
    let occupied = vec![Point { x: 350, y: 1600 }, Point { x: 350, y: 2400 }];
    let mut scorer = Scorer::default();
    for _ in 0..8 {
        scorer.insert(occupied.clone());
    }
    let direct = routing::path_between(start, end, &obstacles, None);
    let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
    assert_eq!(measure(&[occupied.clone(), direct]).crossing_count, 1);
    assert_eq!(measure(&[occupied, detour]).crossing_count, 0);
}

#[test]
fn congested_container_routes_improve_over_shortest_paths() {
    let raw = plan::parse(include_str!(
        "../../../../examples/terraform-large/plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&raw).0;
    let layout = Layout::new(&graph);
    let baseline =
        routing::route_with_quality(&graph, &layout, &ordering::structural_keys(&graph), false);
    let old = measure(&baseline);
    let scored =
        routing::route_with_quality(&graph, &layout, &ordering::structural_keys(&graph), true);
    let new = measure(&scored);
    eprintln!("shortest: {old:?}; scored: {new:?}");
    assert!(new.overlap_distance < old.overlap_distance / 3);
    let cost = |m: &crate::layout::metrics::LayoutMetrics| {
        m.overlap_distance * 8
            + m.crossing_count as u128 * 512
            + m.bend_count as u128 * 96
            + m.total_path_length
    };
    assert!(cost(&new) < cost(&old));
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        let obstacles = routing::obstacles(&layout, edge);
        for pair in path.windows(2) {
            assert!(
                !obstacles
                    .iter()
                    .any(|&b| crate::layout::routing_shared::crosses(pair[0], pair[1], b))
            );
        }
    }
}

#[test]
fn crossing_penalty_selects_a_clear_detour_when_available() {
    use crate::layout::routing_shared::scoring::Scorer;
    let start = Point { x: 100, y: 100 };
    let end = Point { x: 500, y: 100 };
    let obstacles = [
        Bounds {
            origin: Point { x: 200, y: 200 },
            width: 100,
            height: 100,
        },
        Bounds {
            origin: Point { x: 600, y: 140 },
            width: 100,
            height: 100,
        },
    ];
    let occupied = vec![Point { x: 350, y: 80 }, Point { x: 350, y: 150 }];
    let shortest = routing::path_between(start, end, &obstacles, None);
    let mut scorer = Scorer::default();
    scorer.insert(occupied.clone());
    let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
    assert_eq!(measure(&[occupied.clone(), shortest]).crossing_count, 1);
    assert_eq!(measure(&[occupied, detour]).crossing_count, 0);
}

#[test]
fn completed_paths_compact_stubs_before_scoring_and_occupancy() {
    use crate::layout::routing_shared::scoring::Scorer;
    for (start, end, expected, split) in [
        (
            Point { x: 100, y: 100 },
            Point { x: 500, y: 220 },
            vec![
                Point { x: 100, y: 100 },
                Point { x: 516, y: 100 },
                Point { x: 516, y: 220 },
                Point { x: 500, y: 220 },
            ],
            Point { x: 116, y: 100 },
        ),
        (
            Point { x: 500, y: 100 },
            Point { x: 100, y: 220 },
            vec![
                Point { x: 500, y: 100 },
                Point { x: 516, y: 100 },
                Point { x: 516, y: 220 },
                Point { x: 100, y: 220 },
            ],
            Point { x: 116, y: 220 },
        ),
    ] {
        for quality in [false, true] {
            let mut scorer = Scorer::default();
            let path = routing::path_between(start, end, &[], quality.then_some(&scorer));
            assert_eq!(path, expected);
            assert_eq!(scorer.readability_cost(&path), 552 + 2 * 96);
            scorer.insert(path);
            let crossing = [
                Point {
                    x: split.x,
                    y: split.y - 10,
                },
                Point {
                    x: split.x,
                    y: split.y + 10,
                },
            ];
            assert_eq!(scorer.readability_cost(&crossing), 20 + 512);
        }
    }
}

#[test]
fn stub_junction_crossings_select_a_turn_away_detour() {
    use crate::layout::routing_shared::scoring::Scorer;
    for (start, end, junction) in [
        (
            Point { x: 100, y: 100 },
            Point { x: 500, y: 220 },
            Point { x: 116, y: 100 },
        ),
        (
            Point { x: 500, y: 100 },
            Point { x: 100, y: 220 },
            Point { x: 116, y: 220 },
        ),
    ] {
        let obstacles = [Bounds {
            origin: Point { x: 207, y: 148 },
            width: 100,
            height: 60,
        }];
        let mut scorer = Scorer::default();
        scorer.insert(vec![
            Point {
                x: junction.x,
                y: junction.y - 10,
            },
            Point {
                x: junction.x,
                y: junction.y + 10,
            },
        ]);
        let direct = routing::path_between(start, end, &obstacles, Some(&Scorer::default()));
        let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
        assert!(
            scorer.readability_cost(&detour) < scorer.readability_cost(&direct),
            "start={start:?} direct={direct:?} detour={detour:?}"
        );
        assert!(
            scorer.readability_cost(&detour) - Scorer::default().readability_cost(&detour) < 512
        );
    }
}
