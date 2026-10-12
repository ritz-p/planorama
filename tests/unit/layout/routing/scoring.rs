use super::*;

fn points(values: &[(usize, usize)]) -> Vec<Point> {
    values.iter().map(|&(x, y)| Point { x, y }).collect()
}

#[test]
fn frame_cost_penalizes_visible_near_misses_and_keeps_detours_bounded() {
    let frame = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 200,
        height: 200,
    };
    let mut scorer = Scorer::default();
    scorer.set_borders(&[frame]);
    for path in [
        points(&[(120, 100), (280, 100)]),
        points(&[(100, 120), (100, 280)]),
        points(&[(50, 100), (150, 100)]),
        points(&[(120, 100), (124, 100)]),
        points(&[(120, 99), (280, 99)]),
        points(&[(100, 100), (108, 100)]),
    ] {
        let a = path[0];
        let b = path[1];
        let m = Point {
            x: (a.x + b.x) / 2,
            y: (a.y + b.y) / 2,
        };
        let cost = scorer.border_metrics(&path).cost();
        assert!(cost > 0);
        assert_eq!(scorer.segment_cost(a, b), cost);
        assert_eq!(scorer.segment_cost(a, m) + scorer.segment_cost(m, b), cost);
        assert_eq!(scorer.segment_cost(b, a), cost);
        assert_eq!(
            scorer.readability_cost(&path),
            Scorer::default().readability_cost(&path) + cost
        );
    }
    assert_eq!(
        scorer.segment_cost(Point { x: 120, y: 90 }, Point { x: 120, y: 110 }),
        0
    );
    let along = points(&[(120, 100), (280, 100)]);
    let offset = points(&[(120, 100), (120, 90), (280, 90), (280, 100)]);
    assert_eq!(
        scorer.choose(vec![along.clone(), offset.clone()], &[]),
        Some(offset.clone())
    );
    assert!(!scorer.safe_shortcut(&offset, &along, &[]));
    let brief = points(&[(120, 100), (124, 100)]);
    let detour = points(&[(120, 100), (120, 90), (124, 90), (124, 100)]);
    assert_eq!(scorer.choose(vec![detour, brief.clone()], &[]), Some(brief));
    scorer.set_soft(vec![Occluder {
        bounds: frame,
        cost: 1,
        step: 2,
    }]);
    assert_eq!(scorer.segment_cost(along[0], along[1]), 0);
}

#[test]
fn shortcut_guard_and_readonly_kind_scoring_use_visible_stroke_gaps() {
    let frame = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 200,
        height: 200,
    };
    let mut scorer = Scorer::default();
    scorer.set_borders(&[frame]);
    let clear = points(&[(120, 94), (280, 94)]);
    let close = points(&[(120, 96), (280, 96)]);
    assert!(!scorer.safe_border_change(&clear, &close));
    assert!(scorer.safe_border_change(&close, &clear));
    assert!(
        scorer.readability_cost_for(&close, &[], EdgeKind::Connection)
            > scorer.readability_cost_for(&close, &[], EdgeKind::Dependency)
    );
    scorer.set_kind(EdgeKind::Connection);
    assert_eq!(
        scorer.readability_cost(&close),
        scorer.readability_cost_for(&close, &[], EdgeKind::Connection)
    );
    assert_eq!(scorer.border_metrics(&clear).minimum_deficit, 0);
}

#[test]
fn equal_geometry_uses_crossings_as_a_stable_tie_breaker() {
    let clear = points(&[(10, 50), (10, 90), (110, 90), (110, 50)]);
    let crossing = points(&[(10, 50), (10, 10), (110, 10), (110, 50)]);
    for reverse in [false, true] {
        let mut occupied = [points(&[(50, 0), (50, 30)]), points(&[(70, 0), (70, 30)])];
        if reverse {
            occupied.reverse();
        }
        let mut scorer = Scorer::default();
        for path in occupied {
            scorer.insert(path);
        }
        assert!(scorer.score(&clear, &[]) < scorer.score(&crossing, &[]));
        for candidates in [
            vec![clear.clone(), crossing.clone()],
            vec![crossing.clone(), clear.clone()],
        ] {
            assert_eq!(scorer.choose(candidates, &[]), Some(clear.clone()));
        }
        let score = scorer.score(&crossing, &[]);
        let conflicts: u128 = crossing
            .windows(2)
            .map(|p| scorer.segment_cost(p[0], p[1]))
            .sum();
        assert_eq!(
            score.cost,
            score.length + score.bends as u128 * BEND_COST + conflicts
        );
        assert_eq!(conflicts, 2 * CROSSING_COST);
    }
}

#[test]
fn candidate_budget_limits_stretch_even_under_heavy_congestion() {
    let short = points(&[(100, 500), (600, 500)]);
    let long = points(&[
        (100, 500),
        (140, 500),
        (140, 2500),
        (560, 2500),
        (560, 500),
        (600, 500),
    ]);
    let mut scorer = Scorer::default();
    for _ in 0..10000 {
        scorer.insert(points(&[(350, 400), (350, 600)]));
    }
    assert!(scorer.score(&long, &[]) < scorer.score(&short, &[]));
    assert_eq!(scorer.choose(vec![long, short.clone()], &[]), Some(short));
}

#[test]
fn ordinary_crossings_do_not_pay_for_extra_bends_or_detours() {
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
    assert!(scorer.score(&direct, &[]) < scorer.score(&modest, &[]));
    assert!(scorer.safe_shortcut(&extreme, &direct, &[]));
    assert!(scorer.safe_shortcut(&modest, &direct, &[]));
    for _ in 0..7 {
        scorer.insert(points(&[(350, 480), (350, 520)]));
    }
    assert!(scorer.score(&direct, &[]) < scorer.score(&extreme, &[]));
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
    assert_eq!(scorer.segment_cost(a, b), 3 * CROSSING_COST);
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
fn crossings_are_less_expensive_than_bends_and_length() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(50, 20), (50, 80)]));
    let crossing = points(&[(0, 50), (100, 50)]);
    let clear = points(&[(0, 50), (0, 100), (100, 100), (100, 50)]);
    assert_eq!(scorer.score(&crossing, &[]).crossings, 1);
    assert!(scorer.score(&crossing, &[]) < scorer.score(&clear, &[]));
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
        scorer.readability_cost_with_soft(&path, &[card.into()]),
        clear + 80
    );
    scorer.set_soft(vec![card]);
    assert_eq!(scorer.readability_cost_with_soft(&path, &[]), clear);
    assert_eq!(
        scorer.readability_cost_with_soft(&path, &[card.into()]),
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
            assert_eq!(
                scorer.junction_cost(point(50, 50), horizontal),
                CROSSING_COST
            );
        }
    }
}

fn crossing(first: &[Point], second: &[Point]) -> Option<(usize, usize)> {
    let (h, v) = match (first[0].y == first[1].y, second[0].y == second[1].y) {
        (true, false) => (first, second),
        (false, true) => (second, first),
        _ => return None,
    };
    let (x, y) = (v[0].x, h[0].y);
    match x > h[0].x.min(h[1].x)
        && x < h[0].x.max(h[1].x)
        && y > v[0].y.min(v[1].y)
        && y < v[0].y.max(v[1].y)
    {
        true => Some((x, y)),
        false => None,
    }
}
#[test]
fn search_borders_follow_current_container_masks() {
    let outer = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 10000,
        height: 10000,
    };
    let partial = Bounds {
        origin: Point { x: 80, y: 80 },
        width: 100,
        height: 100,
    };
    let mut frames = vec![outer, partial];
    frames.extend((0..1000).map(|i| Bounds {
        origin: Point {
            x: 120 + i * 8,
            y: 120 + i * 8,
        },
        width: 40,
        height: 40,
    }));
    let mut scorer = Scorer::default();
    scorer.set_borders(&frames);
    assert_eq!(scorer.borders().count(), frames.len());
    scorer.set_soft(vec![Occluder {
        bounds: outer,
        cost: 1,
        step: 2,
    }]);
    assert_eq!(scorer.borders().copied().collect::<Vec<_>>(), vec![partial]);
    scorer.set_soft(vec![outer]);
    assert_eq!(scorer.borders().count(), frames.len());
    scorer.set_soft(Vec::<Occluder>::new());
    assert_eq!(scorer.borders().count(), frames.len());
}
