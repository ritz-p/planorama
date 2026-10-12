use super::*;

#[test]
fn route_shortcuts_preserve_configured_frames_and_per_edge_masks() {
    let frame = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 300,
        height: 200,
    };
    let path = vec![
        Point { x: 80, y: 100 },
        Point { x: 96, y: 100 },
        Point { x: 96, y: 84 },
        Point { x: 414, y: 84 },
        Point { x: 414, y: 100 },
        Point { x: 430, y: 100 },
    ];
    let enclosing = Bounds {
        origin: Point { x: 90, y: 90 },
        width: frame.width + 20,
        height: frame.height + 20,
    };
    for (mask, direct) in [(None, false), (Some(frame), false), (Some(enclosing), true)] {
        let mut scorer = Scorer::default();
        scorer.set_borders(&[frame]);
        scorer.insert(vec![Point { x: 0, y: 0 }, Point { x: 500, y: 0 }]);
        let mut paths = vec![path.clone()];
        simplify_routes_with_clearances(
            &mut paths,
            [0],
            |_| Vec::new(),
            |_| {
                mask.into_iter()
                    .map(|bounds| super::super::scoring::Occluder {
                        bounds,
                        cost: 1,
                        step: 2,
                    })
                    .collect()
            },
            |_| [Terminal::plain(16); 2],
            |_| EdgeKind::Connection,
            scorer,
        );
        assert_eq!(paths[0].len() == 2, direct, "{mask:?}: {:?}", paths[0]);
        let mut scorer = Scorer::default();
        scorer.set_borders(&[frame]);
        scorer.set_kind(EdgeKind::Connection);
        scorer.set_soft(
            mask.into_iter()
                .map(|bounds| super::super::scoring::Occluder {
                    bounds,
                    cost: 1,
                    step: 2,
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(scorer.border_metrics(&paths[0]).minimum_deficit, 0);
        let straight = [path[0], *path.last().unwrap()];
        assert_eq!(
            scorer.border_metrics(&straight).minimum_deficit == 0,
            direct
        );
        assert_eq!(paths[0].first(), path.first());
        assert_eq!(paths[0].last(), path.last());
    }
}

#[test]
fn final_shortcuts_keep_a_clear_offset_from_nested_frames() {
    let frame = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 300,
        height: 200,
    };
    let parent = Bounds {
        origin: Point { x: 50, y: 50 },
        width: 400,
        height: 300,
    };
    let path = vec![
        Point { x: 80, y: 100 },
        Point { x: 90, y: 100 },
        Point { x: 90, y: 84 },
        Point { x: 410, y: 84 },
        Point { x: 410, y: 100 },
        Point { x: 430, y: 100 },
    ];
    let mut scorer = Scorer::default();
    scorer.set_borders(&[parent, frame]);
    let shortened = simplify_path(path.clone(), &[], &scorer);
    assert_eq!(shortened.first(), path.first());
    assert_eq!(shortened.last(), path.last());
    assert_eq!(
        crate::layout::metrics::border_overlap(&shortened, &[parent, frame]),
        0
    );
    assert!(shortened.len() > 2);
    let start = Port {
        point: path[0],
        side: Side::Right,
    };
    let end = Port {
        point: *path.last().unwrap(),
        side: Side::Left,
    };
    let routed = ports::connect(start, end, &[], Some(&scorer)).unwrap();
    assert_eq!(
        crate::layout::metrics::border_overlap(&routed, &[parent, frame]),
        0
    );
}

#[test]
fn numbered_shortcuts_remove_middle_doglegs_and_preserve_badge_space() {
    let path: Vec<_> = [
        (100, 100),
        (160, 100),
        (160, 160),
        (260, 160),
        (260, 100),
        (320, 100),
    ]
    .into_iter()
    .map(|(x, y)| Point { x, y })
    .collect();
    let scorer = Scorer::default();
    let terminals = [
        Terminal::marker(10, Side::Right),
        Terminal::marker(10, Side::Left),
    ];
    assert_eq!(
        simplify_path_with_clearances(path.clone(), &[], &scorer, terminals),
        vec![path[0], path[5]]
    );
    let start = Port {
        point: path[0],
        side: Side::Right,
    };
    let end = Port {
        point: Point { x: 200, y: 160 },
        side: Side::Left,
    };
    let short = [
        start.point,
        Point { x: 120, y: 100 },
        Point { x: 120, y: 160 },
        end.point,
    ];
    assert!(ports::valid(&short, start, end, &[]));
    assert!(!ports::valid_with_clearances(
        &short,
        start,
        end,
        &[],
        terminals
    ));
    let routed = ports::connect_with_clearances(start, end, &[], None, terminals).unwrap();
    let simplified = simplify_path_with_clearances(routed, &[], &scorer, terminals);
    assert!(ports::valid_with_clearances(
        &simplified,
        start,
        end,
        &[],
        terminals
    ));
}

#[test]
fn shortcuts_cannot_cut_across_a_vertical_marker_footprint() {
    let start = Port {
        point: Point { x: 200, y: 220 },
        side: Side::Bottom,
    };
    let scorer = Scorer::default();
    for number in [10, 100] {
        let marker = Terminal::marker(number, start.side);
        let end = Port {
            point: Point {
                x: start.point.x + crate::layout::relationship_markers::width(number) - 2,
                y: 230,
            },
            side: Side::Left,
        };
        let lane = start.point.x + marker.span / 2 + 1;
        let path = vec![
            start.point,
            Point { x: 200, y: 280 },
            Point { x: 300, y: 280 },
            Point { x: 300, y: 200 },
            Point { x: lane, y: 200 },
            Point { x: lane, y: 230 },
            end.point,
        ];
        let terminals = [marker, Terminal::plain(8)];
        assert!(ports::valid_with_clearances(
            &path,
            start,
            end,
            &[],
            terminals
        ));
        let unprotected = simplify_path_with_clearances(
            path.clone(),
            &[],
            &scorer,
            [Terminal::plain(marker.clearance), terminals[1]],
        );
        assert!(!ports::valid_with_clearances(
            &unprotected,
            start,
            end,
            &[],
            terminals,
        ));
        let shortened = simplify_path_with_clearances(path.clone(), &[], &scorer, terminals);
        assert!(shortened.len() < path.len());
        assert!(ports::valid_with_clearances(
            &shortened,
            start,
            end,
            &[],
            terminals,
        ));
        let corridor = marker.corridor(start).unwrap();
        assert!(
            shortened
                .windows(2)
                .skip(1)
                .all(|points| { !super::super::crosses(points[0], points[1], corridor) })
        );
    }
}

#[test]
fn ordinary_crossings_allow_shortcuts_and_excluded_paths_are_unchanged() {
    let dogleg = vec![
        Point { x: 100, y: 100 },
        Point { x: 140, y: 100 },
        Point { x: 140, y: 160 },
        Point { x: 260, y: 160 },
        Point { x: 260, y: 100 },
        Point { x: 300, y: 100 },
    ];
    let mut paths = vec![
        dogleg,
        vec![Point { x: 200, y: 80 }, Point { x: 200, y: 120 }],
    ];
    let protected = paths[1].clone();
    simplify_routes(&mut paths, [0], |_| Vec::new());
    assert_eq!(paths[0].len(), 2);
    assert_eq!(paths[1], protected);
    assert_eq!(crate::layout::metrics::measure(&paths).crossing_count, 1);
}

#[test]
fn shortcuts_preserve_ports_and_reject_obstacles_and_conflicts() {
    let path = vec![
        Point { x: 100, y: 100 },
        Point { x: 140, y: 100 },
        Point { x: 140, y: 160 },
        Point { x: 260, y: 160 },
        Point { x: 260, y: 100 },
        Point { x: 300, y: 100 },
    ];
    let scorer = Scorer::default();
    assert_eq!(
        simplify_path(path.clone(), &[], &scorer),
        vec![path[0], path[5]]
    );
    let blocker = Bounds {
        origin: Point { x: 180, y: 80 },
        width: 40,
        height: 60,
    };
    let retained = simplify_path(path.clone(), &[blocker], &scorer);
    assert!(retained.len() > 2);
    assert_eq!(retained.first(), path.first());
    assert_eq!(retained.last(), path.last());
    let mut occupied = Scorer::default();
    occupied.insert(vec![Point { x: 200, y: 80 }, Point { x: 200, y: 120 }]);
    let clear = simplify_path(path.clone(), &[], &occupied);
    assert_eq!(clear.len(), 2);
    assert_eq!(clear, simplify_path(clear.clone(), &[], &occupied));
    let reversed = vec![
        Point { x: 100, y: 100 },
        Point { x: 120, y: 100 },
        Point { x: 120, y: 200 },
        Point { x: 50, y: 200 },
    ];
    let result = simplify_path(reversed.clone(), &[], &scorer);
    assert!(result[1].x > result[0].x);
    assert_eq!(result.first(), reversed.first());
    assert_eq!(result.last(), reversed.last());
    let start = Port {
        point: Point { x: 100, y: 100 },
        side: Side::Right,
    };
    let end = Port {
        point: Point { x: 108, y: 108 },
        side: Side::Top,
    };
    let routed = ports::connect(start, end, &[], None).unwrap();
    let shortened = simplify_path(routed, &[], &scorer);
    assert!(shortened[1].x >= 116);
    assert!(shortened[shortened.len() - 2].y <= 92);
}
