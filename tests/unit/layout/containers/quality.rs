use super::*;
use crate::layout::metrics::measure;

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
    let obstacles = [Bounds {
        origin: Point { x: 200, y: 200 },
        width: 100,
        height: 100,
    }];
    let occupied = vec![Point { x: 350, y: 80 }, Point { x: 350, y: 150 }];
    let shortest = routing::path_between(start, end, &obstacles, None);
    let mut scorer = Scorer::default();
    scorer.insert(occupied.clone());
    let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
    assert_eq!(measure(&[occupied.clone(), shortest]).crossing_count, 1);
    assert_eq!(measure(&[occupied, detour]).crossing_count, 0);
}
