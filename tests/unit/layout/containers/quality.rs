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
}

#[test]
fn coincident_crossings_make_a_long_clear_detour_worthwhile() {
    use crate::layout::routing::scoring::Scorer;
    let start = Point { x: 100, y: 2000 };
    let end = Point { x: 500, y: 2000 };
    let obstacles = [
        Bounds {
            origin: Point { x: 200, y: 4000 },
            width: 100,
            height: 100,
        },
        Bounds {
            origin: Point { x: 600, y: 2040 },
            width: 100,
            height: 100,
        },
    ];
    let occupied = vec![Point { x: 350, y: 500 }, Point { x: 350, y: 3500 }];
    let mut scorer = Scorer::default();
    for _ in 0..4 {
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
    let new = measure(&layout.paths);
    eprintln!("shortest: {old:?}; scored: {new:?}");
    assert!(new.overlap_distance < old.overlap_distance / 4);
    // Separating coincident lines exposes crossings that overlap-only metrics
    // hide. Charge those explicitly rather than claiming crossings decreased.
    let cost = |m: &crate::layout::metrics::LayoutMetrics| {
        m.overlap_distance * 8
            + m.crossing_count as u128 * 2048
            + m.bend_count as u128 * 24
            + m.total_path_length
    };
    assert!(cost(&new) < cost(&old));
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        let obstacles = routing::obstacles(&layout, edge);
        for pair in path.windows(2) {
            assert!(
                !obstacles
                    .iter()
                    .any(|&b| routing::crosses(pair[0], pair[1], b))
            );
        }
    }
}

#[test]
fn crossing_penalty_selects_a_clear_detour_when_available() {
    use crate::layout::routing::scoring::Scorer;
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
    use crate::layout::routing::scoring::Scorer;
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
            assert_eq!(scorer.readability_cost(&path), 552 + 2 * 24);
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
            assert_eq!(scorer.readability_cost(&crossing), 20 + 2048);
        }
    }
}
