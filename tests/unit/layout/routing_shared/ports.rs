use super::*;

#[test]
fn target_side_change_preserves_source_offset() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 200,
        height: 160,
    };
    let target = Bounds {
        origin: Point { x: 600, y: 100 },
        ..source
    };
    let start = source.port(Side::Right, 45);
    let end = target.port(Side::Right, 70);
    let obstacles = [source, target];
    let baseline = connect(start, end, &obstacles, None).unwrap();
    let slots = [100; 4];
    let path = select_with_slots(
        source,
        target,
        baseline,
        &obstacles,
        &Scorer::default(),
        Some((&slots, &slots)),
    );
    assert_eq!(path.first(), Some(&start.point));
    assert_eq!(path.last(), Some(&target.port(Side::Left, 100).point));
}

#[test]
fn unobstructed_facing_pairs_need_one_candidate_and_no_grid_search() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 100,
    };
    for target in [
        Bounds {
            origin: Point { x: 500, y: 100 },
            ..source
        },
        Bounds {
            origin: Point { x: 100, y: 500 },
            ..source
        },
    ] {
        let obstacles = [source, target];
        let baseline = connect(
            source.port(Side::Right, 50),
            target.port(Side::Right, 50),
            &obstacles,
            None,
        )
        .unwrap();
        CANDIDATE_COUNT.set(0);
        search::GRID_SEARCH_COUNT.set(0);
        let selected = select(source, target, baseline, &obstacles, &Scorer::default());
        assert_eq!(CANDIDATE_COUNT.get(), 1);
        assert_eq!(search::GRID_SEARCH_COUNT.get(), 0);
        assert_eq!(
            Scorer::default().readability_cost(&selected),
            distance_bound(source, target)
        );
        CANDIDATE_COUNT.set(0);
        search::GRID_SEARCH_COUNT.set(0);
        let slots = [50; 4];
        assert_eq!(
            select_from_ports(
                source,
                target,
                [source.port(Side::Right, 50), target.port(Side::Right, 50)],
                &obstacles,
                &Scorer::default(),
                (&slots, &slots),
            ),
            Some(selected)
        );
        assert_eq!(CANDIDATE_COUNT.get(), 1);
        assert_eq!(search::GRID_SEARCH_COUNT.get(), 0);
    }
}

#[test]
fn geometry_batches_use_rectangle_projections_before_expanding() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 300,
        height: 300,
    };
    for (x, y, expected) in [
        (500, 350, vec![(Side::Right, Side::Left)]),
        (350, 500, vec![(Side::Bottom, Side::Top)]),
        (
            500,
            500,
            vec![(Side::Right, Side::Left), (Side::Bottom, Side::Top)],
        ),
        (200, 200, vec![]),
    ] {
        let target = Bounds {
            origin: Point { x, y },
            width: 80,
            height: 80,
        };
        let mut batches = pair_batches(source, target);
        assert_eq!(
            batches
                .next()
                .unwrap()
                .into_iter()
                .map(|(_, p)| p)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(batches.next().unwrap().len(), 16 - expected.len());
        assert!(batches.next().is_none());
    }
}

#[test]
fn side_selection_is_independent_of_the_legacy_pair() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 160,
        height: 160,
    };
    for target in [
        Bounds {
            origin: Point { x: 600, y: 100 },
            ..source
        },
        Bounds {
            origin: Point { x: 100, y: 600 },
            ..source
        },
    ] {
        let obstacles = [source, target];
        let mut expected = None;
        for (a, b) in [
            (Side::Right, Side::Right),
            (Side::Left, Side::Left),
            (Side::Top, Side::Bottom),
        ] {
            let baseline =
                connect(source.port(a, 80), target.port(b, 80), &obstacles, None).unwrap();
            let selected = select(source, target, baseline, &obstacles, &Scorer::default());
            if let Some(expected) = &expected {
                assert_eq!(&selected, expected);
            }
            expected = Some(selected);
        }
    }
}

#[test]
fn blocked_facing_ports_choose_a_shorter_valid_alternative() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 100,
    };
    let target = Bounds {
        origin: Point { x: 600, y: 100 },
        ..source
    };
    let blocker = Bounds {
        origin: Point { x: 205, y: 90 },
        width: 30,
        height: 130,
    };
    let obstacles = [source, target, blocker];
    assert!(
        connect(
            source.port(Side::Right, 50),
            target.port(Side::Left, 50),
            &obstacles,
            None
        )
        .is_none()
    );
    let baseline = connect(
        source.port(Side::Left, 50),
        target.port(Side::Right, 50),
        &obstacles,
        None,
    )
    .unwrap();
    let scorer = Scorer::default();
    CANDIDATE_COUNT.set(0);
    search::GRID_SEARCH_COUNT.set(0);
    let selected = select(source, target, baseline.clone(), &obstacles, &scorer);
    assert!(CANDIDATE_COUNT.get() > 1);
    assert!(search::GRID_SEARCH_COUNT.get() > 0);
    assert!(scorer.readability_cost(&selected) < scorer.readability_cost(&baseline));
    assert!(
        selected
            .windows(2)
            .all(|p| obstacles.iter().all(|&b| !crosses(p[0], p[1], b)))
    );
}

#[test]
fn bounded_selection_matches_exhaustive_candidate_cost() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 100,
    };
    let target = Bounds {
        origin: Point { x: 500, y: 300 },
        ..source
    };
    for blocker in [
        Bounds {
            origin: Point { x: 250, y: 120 },
            width: 100,
            height: 250,
        },
        Bounds {
            origin: Point { x: 350, y: 210 },
            width: 120,
            height: 40,
        },
    ] {
        let obstacles = [source, target, blocker];
        let mut scorer = Scorer::default();
        scorer.insert(vec![Point { x: 80, y: 220 }, Point { x: 650, y: 220 }]);
        let baseline = connect(
            source.port(Side::Right, 50),
            target.port(Side::Right, 50),
            &obstacles,
            None,
        )
        .unwrap();
        let selected = select(source, target, baseline, &obstacles, &scorer);
        let mut all = Vec::new();
        for (a, b) in candidate_pairs(source, target) {
            let start = source.port(a, 50);
            let end = target.port(b, 50);
            for path in candidates_impl::<false>(start, end, &obstacles, Some(&scorer), [16; 2]) {
                assert!(
                    cost_bound(start, end, [16; 2]) <= Scorer::default().readability_cost(&path)
                );
                all.push(path);
            }
        }
        let exhaustive = scorer.choose(all, &obstacles).unwrap();
        assert_eq!(
            scorer.score(&selected, &obstacles),
            scorer.score(&exhaustive, &obstacles)
        );
    }
}

#[test]
fn grid_pruning_preserves_exhaustive_quality_and_marker_clearance() {
    let start = Port {
        point: Point { x: 200, y: 200 },
        side: Side::Right,
    };
    for end in [
        Port {
            point: Point { x: 600, y: 200 },
            side: Side::Left,
        },
        Port {
            point: Point { x: 600, y: 400 },
            side: Side::Left,
        },
        Port {
            point: Point { x: 600, y: 400 },
            side: Side::Top,
        },
    ] {
        for clearances in [[16, 16], [16, 60], [60, 60]] {
            for occupied in [false, true] {
                let mut scorer = Scorer::default();
                if occupied {
                    scorer.insert(vec![Point { x: 400, y: 100 }, Point { x: 400, y: 300 }]);
                    scorer.set_soft(vec![Bounds {
                        origin: Point { x: 480, y: 180 },
                        width: 40,
                        height: 60,
                    }]);
                }
                let selected = scorer
                    .choose(
                        candidates_impl::<true>(start, end, &[], Some(&scorer), clearances),
                        &[],
                    )
                    .unwrap();
                let exhaustive = scorer
                    .choose(
                        candidates_impl::<false>(start, end, &[], Some(&scorer), clearances),
                        &[],
                    )
                    .unwrap();
                assert!(valid_with_clearances(
                    &selected,
                    start,
                    end,
                    &[],
                    clearances
                ));
                assert_eq!(scorer.score(&selected, &[]), scorer.score(&exhaustive, &[]));
            }
        }
    }
}

#[test]
fn soft_cards_allow_large_savings_but_prefer_small_detours() {
    let start = Port {
        point: Point { x: 100, y: 500 },
        side: Side::Right,
    };
    let end = Port {
        point: Point { x: 600, y: 500 },
        side: Side::Left,
    };
    for (height, crossed) in [(40, false), (900, true)] {
        let card = Bounds {
            origin: Point {
                x: 300,
                y: 500 - height / 2,
            },
            width: 240,
            height,
        };
        let mut scorer = Scorer::default();
        scorer.set_soft(vec![card]);
        let path = connect(start, end, &[], Some(&scorer)).unwrap();
        assert_eq!(
            path.windows(2).any(|p| crosses(p[0], p[1], card)),
            crossed,
            "{path:?}"
        );
        let shortened = super::super::shortcuts::simplify_path(path, &[], &scorer);
        assert_eq!(
            shortened.windows(2).any(|p| crosses(p[0], p[1], card)),
            crossed
        );
        let hard = connect(start, end, &[card], Some(&scorer)).unwrap();
        let hard = super::super::shortcuts::simplify_path(hard, &[card], &scorer);
        assert!(hard.windows(2).all(|p| !crosses(p[0], p[1], card)));
    }
}

#[test]
fn chooses_facing_sides_for_horizontal_vertical_and_container_bounds() {
    let card = |x, y| Bounds {
        origin: Point { x, y },
        width: 100,
        height: 80,
    };
    for (source, target, from_side, to_side) in [
        (card(100, 100), card(350, 100), Side::Right, Side::Left),
        (card(350, 100), card(100, 100), Side::Left, Side::Right),
        (card(100, 100), card(100, 350), Side::Bottom, Side::Top),
        (card(100, 350), card(100, 100), Side::Top, Side::Bottom),
        (
            Bounds {
                width: 220,
                height: 200,
                ..card(100, 100)
            },
            card(500, 100),
            Side::Right,
            Side::Left,
        ),
        (
            card(500, 100),
            Bounds {
                width: 220,
                height: 200,
                ..card(100, 100)
            },
            Side::Left,
            Side::Right,
        ),
    ] {
        let obstacles = [source, target];
        let baseline = connect(
            source.port(Side::Right, source.height / 2),
            target.port(Side::Right, target.height / 2),
            &obstacles,
            None,
        )
        .unwrap();
        let chosen = select(
            source,
            target,
            baseline.clone(),
            &obstacles,
            &Scorer::default(),
        );
        let offset = |bounds: Bounds, side| {
            if matches!(side, Side::Left | Side::Right) {
                bounds.height / 2
            } else {
                bounds.width / 2
            }
        };
        if source.width == target.width && source.height == target.height {
            assert_eq!(
                chosen.first(),
                Some(&source.port(from_side, offset(source, from_side)).point)
            );
            assert_eq!(
                chosen.last(),
                Some(&target.port(to_side, offset(target, to_side)).point)
            );
        } else {
            assert!(
                Scorer::default().readability_cost(&chosen)
                    <= Scorer::default().readability_cost(&baseline)
            );
        }
        assert_eq!(
            chosen,
            select(source, target, baseline, &obstacles, &Scorer::default())
        );
        assert!(
            chosen
                .windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }
}

#[test]
fn side_selection_preserves_obstacle_avoidance() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 80,
    };
    let target = Bounds {
        origin: Point { x: 500, y: 100 },
        ..source
    };
    let obstacle = Bounds {
        origin: Point { x: 280, y: 70 },
        width: 100,
        height: 160,
    };
    let obstacles = [source, target, obstacle];
    let baseline = connect(
        source.port(Side::Right, 40),
        target.port(Side::Right, 40),
        &obstacles,
        None,
    )
    .unwrap();
    let chosen = select(source, target, baseline, &obstacles, &Scorer::default());
    assert!(
        chosen
            .windows(2)
            .all(|p| obstacles.iter().all(|&b| !crosses(p[0], p[1], b)))
    );
}

#[test]
fn perpendicular_pair_beats_equal_quality_facing_pairs() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 80,
        height: 80,
    };
    let target = Bounds {
        origin: Point { x: 300, y: 300 },
        ..source
    };
    let obstacles = [source, target];
    let scorer = Scorer::default();
    let baseline = connect(
        source.port(Side::Bottom, 40),
        target.port(Side::Top, 40),
        &obstacles,
        None,
    )
    .unwrap();
    let preferred = connect(
        source.port(Side::Right, 40),
        target.port(Side::Left, 40),
        &obstacles,
        None,
    )
    .unwrap();
    assert_eq!(
        scorer.score(&baseline, &obstacles),
        scorer.score(&preferred, &obstacles)
    );
    let chosen = select(source, target, baseline, &obstacles, &scorer);
    assert_eq!(chosen.first(), Some(&source.port(Side::Right, 40).point));
    assert_eq!(chosen.last(), Some(&target.port(Side::Top, 40).point));
    assert!(scorer.readability_cost(&chosen) < scorer.readability_cost(&preferred));
}
