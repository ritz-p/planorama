use super::{Bounds, Point, Scorer, Simplification, ports, simple, simplify};
use crate::layout::{Port, Side};

#[cfg(test)]
pub(in crate::layout) fn simplify_routes(
    paths: &mut [Vec<Point>],
    order: impl IntoIterator<Item = usize>,
    obstacles: impl Fn(usize) -> Vec<Bounds>,
) {
    simplify_routes_with_clearances(paths, order, obstacles, |_| Vec::new(), |_| [16; 2], &[]);
}

pub(in crate::layout) fn simplify_routes_with_clearances(
    paths: &mut [Vec<Point>],
    order: impl IntoIterator<Item = usize>,
    obstacles: impl Fn(usize) -> Vec<Bounds>,
    soft: impl Fn(usize) -> Vec<super::scoring::Occluder>,
    clearances: impl Fn(usize) -> [usize; 2],
    borders: &[Bounds],
) {
    let mut scorer = Scorer::default();
    scorer.set_borders(borders);
    for path in paths.iter() {
        scorer.insert(path.clone());
    }
    for index in order {
        if paths[index].len() < 3 {
            continue;
        }
        scorer.remove(index);
        scorer.set_soft(soft(index));
        paths[index] = simplify_path_with_clearances(
            std::mem::take(&mut paths[index]),
            &obstacles(index),
            &scorer,
            clearances(index),
        );
        scorer.replace(index, paths[index].clone());
    }
}

pub(in crate::layout) fn simplify_path(
    path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    simplify_path_with_clearances(path, obstacles, scorer, [16; 2])
}

pub(in crate::layout) fn simplify_path_with_clearances(
    mut path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
    clearances: [usize; 2],
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
                    if ports::valid_with_clearances(&candidate, start, end, obstacles, clearances)
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

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/shortcuts.rs"]
mod tests;
