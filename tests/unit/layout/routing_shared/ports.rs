use super::*;

#[test]
fn actual_marker_extent_keeps_short_facing_connections_straight() {
    for (start, end, old_clearance, length) in [
        (
            Port {
                point: Point { x: 200, y: 220 },
                side: Side::Bottom,
            },
            Port {
                point: Point { x: 200, y: 292 },
                side: Side::Top,
            },
            42,
            72,
        ),
        (
            Port {
                point: Point { x: 200, y: 220 },
                side: Side::Right,
            },
            Port {
                point: Point { x: 284, y: 220 },
                side: Side::Left,
            },
            50,
            84,
        ),
    ] {
        let terminals = [start, end].map(|port| Terminal::marker(1, port.side));
        let oversized = terminals.map(|terminal| Terminal {
            clearance: old_clearance,
            ..terminal
        });
        let direct = vec![start.point, end.point];
        assert!(!valid_with_clearances(&direct, start, end, &[], oversized));
        search::GRID_SEARCH_COUNT.set(0);
        assert_eq!(
            connect_with_clearances(start, end, &[], None, terminals),
            Some(direct)
        );
        assert_eq!(cost_bound(start, end, terminals), length);
        assert!(cost_bound(start, end, oversized) > length);
        assert_eq!(search::GRID_SEARCH_COUNT.get(), 0);
    }
}

#[test]
fn vertical_markers_protect_their_full_width_and_preserve_obstacle_detours() {
    let start = Port {
        point: Point { x: 200, y: 220 },
        side: Side::Bottom,
    };
    for number in [10, 100] {
        let terminal = Terminal::marker(number, start.side);
        let lateral = start.point.x + crate::layout::relationship_markers::width(number) / 2 - 1;
        let end = Port {
            point: Point { x: 500, y: 300 },
            side: Side::Left,
        };
        let reentry = [
            start.point,
            Point { x: 200, y: 280 },
            Point { x: lateral, y: 280 },
            Point { x: lateral, y: 230 },
            Point { x: 440, y: 230 },
            Point { x: 440, y: 300 },
            end.point,
        ];
        assert!(valid_with_clearance(&reentry, start, end, &[], 50));
        assert!(!valid_with_clearances(
            &reentry,
            start,
            end,
            &[],
            [terminal, Terminal::plain(50)],
        ));

        let end = Port {
            point: Point { x: 200, y: 400 },
            side: Side::Top,
        };
        let blocker = Bounds {
            origin: Point { x: 190, y: 290 },
            width: 20,
            height: 30,
        };
        let terminals = [terminal, Terminal::marker(number, end.side)];
        let path = connect_with_clearances(start, end, &[blocker], None, terminals).unwrap();
        assert!(path.len() > 2);
        assert!(valid_with_clearances(
            &path,
            start,
            end,
            &[blocker],
            terminals
        ));
        assert!(
            path.windows(2)
                .all(|points| !crosses(points[0], points[1], blocker))
        );
        assert!(
            path[1..path.len() - 1]
                .iter()
                .any(|point| { point.x.abs_diff(start.point.x) >= terminal.span / 2 })
        );
    }
    assert!(terminal_corridors(start, start, [Terminal::plain(50); 2]).is_empty());
}

#[test]
fn terminal_approaches_cannot_cross_the_other_end_marker() {
    let start = Port {
        point: Point { x: 250, y: 100 },
        side: Side::Bottom,
    };
    let end = Port {
        point: Point { x: 260, y: 240 },
        side: Side::Left,
    };
    let path = vec![
        start.point,
        Point { x: 250, y: 300 },
        Point { x: 180, y: 300 },
        Point { x: 180, y: 240 },
        end.point,
    ];
    let terminals = [Terminal::plain(16), Terminal::marker(100, end.side)];
    assert!(valid(&path, start, end, &[]));
    assert!(!valid_with_clearances(&path, start, end, &[], terminals));
    let reversed = path.into_iter().rev().collect::<Vec<_>>();
    assert!(!valid_with_clearances(
        &reversed,
        end,
        start,
        &[],
        [terminals[1], terminals[0]],
    ));
}

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
            for path in candidates_impl::<false>(
                start,
                end,
                &obstacles,
                Some(&scorer),
                [Terminal::plain(16); 2],
            ) {
                assert!(
                    cost_bound(start, end, [Terminal::plain(16); 2])
                        <= Scorer::default().readability_cost(&path)
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
        for terminals in [
            [Terminal::plain(16); 2],
            [Terminal::plain(16), Terminal::marker(100, end.side)],
            [
                Terminal::marker(100, start.side),
                Terminal::marker(100, end.side),
            ],
        ] {
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
                        candidates_impl::<true>(start, end, &[], Some(&scorer), terminals),
                        &[],
                    )
                    .unwrap();
                let exhaustive = scorer
                    .choose(
                        candidates_impl::<false>(start, end, &[], Some(&scorer), terminals),
                        &[],
                    )
                    .unwrap();
                assert!(valid_with_clearances(&selected, start, end, &[], terminals));
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
