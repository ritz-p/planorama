use super::*;

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
    assert_eq!(
        simplify_path_with_clearances(path.clone(), &[], &scorer, [50; 2]),
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
        [50; 2]
    ));
    let routed = ports::connect_with_clearances(start, end, &[], None, [50; 2]).unwrap();
    let simplified = simplify_path_with_clearances(routed, &[], &scorer, [50; 2]);
    assert!(ports::valid_with_clearances(
        &simplified,
        start,
        end,
        &[],
        [50; 2]
    ));
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
