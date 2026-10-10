use super::*;

fn points(values: &[(usize, usize)]) -> Vec<Point> {
    values.iter().map(|&(x, y)| Point { x, y }).collect()
}

#[test]
fn crossing_avoidance_has_a_finite_length_and_bend_budget() {
    let direct = points(&[(100, 500), (600, 500)]);
    let modest = points(&[
        (100, 500),
        (140, 500),
        (140, 550),
        (560, 550),
        (560, 500),
        (600, 500),
    ]);
    let extreme = points(&[
        (100, 500),
        (140, 500),
        (140, 2000),
        (560, 2000),
        (560, 500),
        (600, 500),
    ]);
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(350, 480), (350, 520)]));
    assert!(scorer.score(&direct, &[]) < scorer.score(&extreme, &[]));
    assert!(scorer.score(&modest, &[]) < scorer.score(&direct, &[]));
    assert!(scorer.safe_shortcut(&extreme, &direct, &[]));
    assert!(!scorer.safe_shortcut(&modest, &direct, &[]));
    for _ in 0..7 {
        scorer.insert(points(&[(350, 480), (350, 520)]));
    }
    assert!(scorer.score(&extreme, &[]) < scorer.score(&direct, &[]));
    let zigzag: Vec<_> = (0..24)
        .map(|i| Point {
            x: 100 + (i / 2) * 40,
            y: 550 + (i % 4 / 2) * 10,
        })
        .collect();
    let scorer = Scorer::default();
    assert!(scorer.score(&direct, &[]) < scorer.score(&zigzag, &[]));
}

#[test]
fn replacing_routes_matches_a_fresh_index() {
    let mut paths = vec![
        points(&[(10, 50), (90, 50), (10, 50)]),
        points(&[(50, 10), (50, 90), (50, 10)]),
        points(&[(20, 50), (80, 50), (80, 100)]),
    ];
    let mut scorer = Scorer::default();
    for path in &paths {
        scorer.insert(path.clone());
    }
    let queries = [
        points(&[(0, 50), (100, 50)]),
        points(&[(50, 0), (50, 100)]),
        points(&[(0, 30), (100, 30), (100, 70), (0, 70)]),
    ];
    for index in [0, 1, 2, 0] {
        scorer.remove(index);
        let mut fresh = Scorer::default();
        for (other, path) in paths.iter().enumerate() {
            if other != index {
                fresh.insert(path.clone());
            }
        }
        for query in &queries {
            assert_eq!(scorer.score(query, &[]), fresh.score(query, &[]));
            let expected: usize = fresh
                .paths
                .iter()
                .map(|path| {
                    query
                        .windows(2)
                        .flat_map(|a| path.windows(2).filter_map(move |b| crossing(a, b)))
                        .collect::<BTreeSet<_>>()
                        .len()
                })
                .sum();
            assert_eq!(scorer.score(query, &[]).crossings, expected);
            for pair in query.windows(2) {
                assert_eq!(
                    scorer.segment_cost(pair[0], pair[1]),
                    fresh.segment_cost(pair[0], pair[1])
                );
            }
        }
        paths[index] = queries[index].clone();
        scorer.replace(index, paths[index].clone());
        assert_eq!(scorer.paths.len(), paths.len());
    }
}

#[test]
fn grid_crossings_preserve_path_multiplicity_but_not_repeated_segments() {
    let mut scorer = Scorer::default();
    for _ in 0..3 {
        scorer.insert(points(&[(50, 10), (50, 90), (50, 10)]));
    }
    let a = Point { x: 10, y: 50 };
    let b = Point { x: 90, y: 50 };
    assert_eq!(scorer.score(&[a, b], &[]).crossings, 3);
    assert_eq!(scorer.segment_cost(a, b), 3 * 512);
    assert_eq!(
        scorer.segment_cost(Point { x: 10, y: 10 }, Point { x: 90, y: 10 }),
        0
    );
}

#[test]
fn grid_cost_counts_crossings_at_subdivision_vertices_and_occupied_segments() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(50, 10), (50, 90)]));
    let full = scorer.segment_cost(Point { x: 10, y: 50 }, Point { x: 90, y: 50 });
    let split = scorer.segment_cost(Point { x: 10, y: 50 }, Point { x: 50, y: 50 })
        + scorer.segment_cost(Point { x: 50, y: 50 }, Point { x: 90, y: 50 })
        + scorer.junction_cost(Point { x: 50, y: 50 }, true);
    assert!(full > 0);
    assert_eq!(full, split);
    assert_eq!(
        full,
        scorer.segment_cost(Point { x: 90, y: 50 }, Point { x: 10, y: 50 })
    );
    assert!(scorer.segment_cost(Point { x: 50, y: 20 }, Point { x: 50, y: 80 }) > 0);
    assert_eq!(
        scorer.segment_cost(Point { x: 10, y: 100 }, Point { x: 90, y: 100 }),
        0
    );
}

#[test]
fn longer_clear_route_beats_shorter_overlapping_route() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(20, 50), (80, 50)]));
    let short = points(&[(0, 50), (100, 50)]);
    let clear = points(&[(0, 50), (0, 100), (100, 100), (100, 50)]);
    assert!(scorer.score(&clear, &[]) < scorer.score(&short, &[]));
}

#[test]
fn crossings_are_more_expensive_than_bends_and_length() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(50, 20), (50, 80)]));
    let crossing = points(&[(0, 50), (100, 50)]);
    let clear = points(&[(0, 50), (0, 100), (100, 100), (100, 50)]);
    assert_eq!(scorer.score(&crossing, &[]).crossings, 1);
    assert!(scorer.score(&clear, &[]) < scorer.score(&crossing, &[]));
}

#[test]
fn card_interiors_take_priority_over_edge_overlap() {
    let mut scorer = Scorer::default();
    let around = points(&[(0, 150), (0, 250), (500, 250), (500, 150)]);
    scorer.insert(around.clone());
    let through = points(&[(0, 150), (500, 150)]);
    let nodes = [Bounds::card(Point { x: 100, y: 100 })];
    assert!(scorer.score(&around, &nodes) < scorer.score(&through, &nodes));
}

#[test]
fn overlap_is_direction_independent_and_contacts_do_not_cross() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(10, 20), (100, 20)]));
    assert_eq!(
        scorer.score(&points(&[(80, 20), (30, 20)]), &[]).overlap,
        50
    );
    assert_eq!(
        scorer.score(&points(&[(50, 20), (50, 80)]), &[]).crossings,
        0
    );
    assert_eq!(
        scorer.score(&points(&[(50, 20), (50, 80)]), &[]),
        scorer.score(&points(&[(50, 20), (50, 80)]), &[])
    );
}

#[test]
fn equal_overlap_and_crossings_prefer_fewer_bends_then_shorter_paths() {
    let scorer = Scorer::default();
    let direct = points(&[(0, 10), (100, 10)]);
    let detour = points(&[(0, 10), (0, 30), (100, 30), (100, 10)]);
    assert!(scorer.score(&direct, &[]) < scorer.score(&detour, &[]));
    assert!(scorer.score(&direct, &[]) < scorer.score(&points(&[(0, 10), (200, 10)]), &[]));
}

#[test]
fn branch_soft_cost_uses_its_own_endpoint_exclusions() {
    let mut scorer = Scorer::default();
    let path = points(&[(0, 50), (100, 50)]);
    let card = Bounds {
        origin: Point { x: 20, y: 20 },
        width: 40,
        height: 60,
    };
    let clear = scorer.readability_cost(&path);
    assert_eq!(
        scorer.readability_cost_with_soft(&path, &[card]),
        clear + 80
    );
    scorer.set_soft(vec![card]);
    assert_eq!(scorer.readability_cost_with_soft(&path, &[]), clear);
    assert_eq!(
        scorer.readability_cost_with_soft(&path, &[card]),
        clear + 80
    );
}

#[test]
fn bend_contacts_have_no_crossing_cost_in_either_direction_or_axis() {
    for horizontal in [true, false] {
        let point = |x, y| {
            if horizontal {
                Point { x, y }
            } else {
                Point { x: y, y: x }
            }
        };
        let mut scorer = Scorer::default();
        scorer.insert(vec![point(50, 10), point(50, 90)]);
        for x in [10, 90] {
            let approach = scorer.segment_cost(point(x, 50), point(50, 50));
            let departure = scorer.segment_cost(point(50, 50), point(50, 100));
            let path = [point(x, 50), point(50, 50), point(50, 100)];
            let score = scorer.score(&path, &[]);
            assert_eq!(score.crossings, 0);
            assert_eq!(approach + departure, score.overlap * 8);
            assert_eq!(scorer.junction_cost(point(50, 50), horizontal), 512);
        }
    }
}
