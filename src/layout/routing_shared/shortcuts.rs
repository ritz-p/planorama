use super::{Bounds, Point, Scorer, Simplification, ports, simple, simplify};
use crate::layout::{Port, Side};

pub(in crate::layout) fn simplify_routes(
    paths: &mut [Vec<Point>],
    order: impl IntoIterator<Item = usize>,
    obstacles: impl Fn(usize) -> Vec<Bounds>,
) {
    let mut scorer = Scorer::default();
    for path in paths.iter() {
        scorer.insert(path.clone());
    }
    for index in order {
        if paths[index].len() < 3 {
            continue;
        }
        scorer.remove(index);
        paths[index] = simplify_path(
            std::mem::take(&mut paths[index]),
            &obstacles(index),
            &scorer,
        );
        scorer.replace(index, paths[index].clone());
    }
}

pub(in crate::layout) fn simplify_path(
    mut path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    path = simplify(path, Simplification::PreserveReversals);
    if path.len() < 3 {
        return path;
    }
    let port = |a: Point, b: Point| Port {
        point: a,
        side: match a.x.cmp(&b.x) {
            std::cmp::Ordering::Equal => {
                if b.y < a.y {
                    Side::Top
                } else {
                    Side::Bottom
                }
            }
            std::cmp::Ordering::Greater => Side::Left,
            std::cmp::Ordering::Less => Side::Right,
        },
    };
    let start = port(path[0], path[1]);
    let end = port(path[path.len() - 1], path[path.len() - 2]);
    loop {
        let mut replacement = None;
        'scan: for from in 0..path.len().saturating_sub(2) {
            for to in (from + 2..path.len()).rev() {
                for middle in simple::candidates(path[from], path[to]) {
                    let candidate = simplify(
                        path[..from]
                            .iter()
                            .copied()
                            .chain(middle)
                            .chain(path[to + 1..].iter().copied()),
                        Simplification::PreserveReversals,
                    );
                    if ports::valid(&candidate, start, end, obstacles)
                        && candidate.windows(3).all(|p| {
                            !((p[0].x == p[1].x && p[1].x == p[2].x)
                                || (p[0].y == p[1].y && p[1].y == p[2].y))
                        })
                        && scorer.safe_shortcut(&path, &candidate, obstacles)
                    {
                        replacement = Some(candidate);
                        break 'scan;
                    }
                }
            }
        }
        match replacement {
            Some(next) => path = next,
            None => return path,
        }
    }
}

#[test]
fn completed_later_paths_block_shortcuts_and_excluded_paths_are_unchanged() {
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
    assert!(paths[0].len() > 2);
    assert_eq!(paths[1], protected);
    assert_eq!(crate::layout::metrics::measure(&paths).crossing_count, 0);
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
    assert!(clear.len() > 2);
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
