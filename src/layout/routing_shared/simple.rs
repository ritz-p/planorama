use super::{Point, Simplification, simplify};

pub(super) fn candidates(start: Point, end: Point) -> Vec<Vec<Point>> {
    let x = start.x.min(end.x) + start.x.abs_diff(end.x) / 2;
    let y = start.y.min(end.y) + start.y.abs_diff(end.y) / 2;
    [
        vec![start, end],
        vec![
            start,
            Point {
                x: end.x,
                y: start.y,
            },
            end,
        ],
        vec![
            start,
            Point {
                x: start.x,
                y: end.y,
            },
            end,
        ],
        vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end],
        vec![start, Point { x: start.x, y }, Point { x: end.x, y }, end],
    ]
    .into_iter()
    .map(|path| simplify(path, Simplification::PreserveReversals))
    .collect()
}

#[test]
fn simple_routes_respect_ports_obstacles_and_edge_conflicts() {
    use super::{
        Scorer,
        ports::{connect, valid},
    };
    use crate::layout::{Bounds, Port, Side};
    let port = |x, y, side| Port {
        point: Point { x, y },
        side,
    };
    for (a, b, len) in [
        (port(100, 100, Side::Right), port(300, 100, Side::Left), 2),
        (port(100, 100, Side::Bottom), port(100, 300, Side::Top), 2),
        (port(100, 100, Side::Right), port(300, 300, Side::Top), 3),
    ] {
        let path = connect(a, b, &[], None).unwrap();
        assert_eq!(path.len(), len);
        assert!(valid(&path, a, b, &[]));
    }
    let a = port(100, 100, Side::Right);
    let b = port(500, 100, Side::Left);
    let obstacles = [Bounds {
        origin: Point { x: 250, y: 60 },
        width: 100,
        height: 80,
    }];
    let detour = connect(a, b, &obstacles, None).unwrap();
    assert!(detour.len() > 2);
    assert!(valid(&detour, a, b, &obstacles));
    let mut scorer = Scorer::default();
    scorer.insert(vec![Point { x: 200, y: 100 }, Point { x: 400, y: 100 }]);
    let endpoints = [
        Bounds {
            origin: Point { x: 0, y: 60 },
            width: 100,
            height: 80,
        },
        Bounds {
            origin: Point { x: 500, y: 60 },
            width: 100,
            height: 80,
        },
    ];
    let clear = connect(a, b, &endpoints, Some(&scorer)).unwrap();
    assert!(!scorer.overlaps(&clear));
    assert!(clear.len() > 2);
}

#[test]
fn peer_detours_prefer_below_unless_safety_or_conflicts_override() {
    use super::{
        Scorer,
        ports::{connect, valid},
    };
    use crate::layout::{Bounds, Port, Side};
    let a = Port {
        point: Point { x: 100, y: 100 },
        side: Side::Right,
    };
    let b = Port {
        point: Point { x: 500, y: 100 },
        side: Side::Left,
    };
    let peer = Bounds {
        origin: Point { x: 250, y: 60 },
        width: 100,
        height: 80,
    };
    let mut scorer = Scorer::default();
    scorer.set_peers(vec![peer]);
    let lower = connect(a, b, &[peer], Some(&scorer)).unwrap();
    assert!(lower.iter().any(|p| p.y > 140), "{lower:?}");
    let blocked = [
        peer,
        Bounds {
            origin: Point { x: 110, y: 140 },
            width: 380,
            height: 100,
        },
    ];
    let upper = connect(a, b, &blocked, Some(&scorer)).unwrap();
    assert!(upper.iter().any(|p| p.y < 60));
    assert!(valid(&upper, a, b, &blocked));
    scorer.insert(vec![Point { x: 200, y: 140 }, Point { x: 200, y: 200 }]);
    let clear = connect(a, b, &[peer], Some(&scorer)).unwrap();
    assert!(clear.iter().any(|p| p.y < 60), "{clear:?}");
}
