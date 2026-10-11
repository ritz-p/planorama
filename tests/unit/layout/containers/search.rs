use super::*;

#[test]
fn initial_stub_bend_can_outweigh_a_short_overlap() {
    let start = Point { x: 116, y: 100 };
    let end = Point { x: 516, y: 220 };
    let obstacles = [Bounds {
        origin: Point { x: 207, y: 148 },
        width: 100,
        height: 60,
    }];
    let mut scorer = Scorer::default();
    scorer.insert(vec![start, Point { x: 119, y: 100 }]);

    assert_eq!(
        find_path(start, end, &obstacles, Some(&scorer)),
        vec![start, Point { x: 516, y: 100 }, end]
    );
}

#[test]
fn terminal_stub_bend_is_charged_before_selecting_the_destination() {
    let start = Point { x: 516, y: 100 };
    let end = Point { x: 116, y: 210 };
    let obstacles = [Bounds {
        origin: Point { x: 308, y: 117 },
        width: 100,
        height: 60,
    }];
    let mut scorer = Scorer::default();
    scorer.insert(vec![end, Point { x: 120, y: 210 }]);

    assert_eq!(
        find_path(start, end, &obstacles, Some(&scorer)),
        vec![start, Point { x: 516, y: 210 }, end]
    );
}

#[test]
fn horizontal_stub_crossings_do_not_add_an_avoidable_bend() {
    for (start, end, junction, source) in [
        (
            Point { x: 116, y: 100 },
            Point { x: 516, y: 220 },
            Point { x: 116, y: 100 },
            true,
        ),
        (
            Point { x: 516, y: 100 },
            Point { x: 116, y: 220 },
            Point { x: 116, y: 220 },
            false,
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
        let path = find_path(start, end, &obstacles, Some(&scorer));
        let segment = if source {
            &path[..2]
        } else {
            &path[path.len() - 2..]
        };
        assert_eq!(
            segment[0].y, segment[1].y,
            "continue through the crossing: {path:?}"
        );
    }
}

#[test]
fn shortest_reference_includes_required_hard_obstacle_detours() {
    let start = Point { x: 100, y: 200 };
    let end = Point { x: 500, y: 200 };
    let barrier = Bounds {
        origin: Point { x: 250, y: 100 },
        width: 100,
        height: 200,
    };
    let path = shortest(start, end, &[barrier], (Side::Right, Side::Left)).unwrap();
    assert!(
        path.windows(2)
            .all(|p| !super::super::crosses(p[0], p[1], barrier))
    );
    assert_eq!(
        crate::layout::metrics::measure(&[path]).total_path_length,
        600
    );
}
