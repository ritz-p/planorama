use super::{Bounds, Point, Scorer, Simplification, ports, simple, simplify};
use crate::layout::{Port, Side, relationship_markers::Terminal};
use crate::model::EdgeKind;

#[path = "../../../tests/experiments/shortcuts/selection.rs"]
mod selection;

#[cfg(test)]
pub(in crate::layout) fn simplify_routes(
    paths: &mut [Vec<Point>],
    order: impl IntoIterator<Item = usize>,
    obstacles: impl Fn(usize) -> Vec<Bounds>,
) {
    simplify_routes_with_clearances(
        paths,
        order,
        obstacles,
        |_| Vec::new(),
        |_| [Terminal::plain(16); 2],
        |_| EdgeKind::Dependency,
        Scorer::default(),
    );
}

pub(in crate::layout) fn simplify_routes_with_clearances(
    paths: &mut [Vec<Point>],
    order: impl IntoIterator<Item = usize>,
    obstacles: impl Fn(usize) -> Vec<Bounds>,
    soft: impl Fn(usize) -> Vec<super::scoring::Occluder>,
    terminals: impl Fn(usize) -> [Terminal; 2],
    kinds: impl Fn(usize) -> EdgeKind,
    mut scorer: Scorer,
) {
    scorer.truncate(0);
    for path in paths.iter() {
        scorer.insert(path.clone());
    }
    let bounded = std::env::var_os("PLANORAMA_BOUNDED").is_some();
    let trace = std::env::var_os("PLANORAMA_SHORTCUT_TRACE").is_some();
    let mut evaluated = 0;
    for index in order {
        if paths[index].len() < 3 {
            continue;
        }
        scorer.remove(index);
        scorer.set_kind(kinds(index));
        scorer.set_soft(soft(index));
        paths[index] = select_shortcuts(
            std::mem::take(&mut paths[index]),
            &obstacles(index),
            &scorer,
            terminals(index),
            if bounded { 8 } else { 1 },
            &mut evaluated,
            trace,
        );
        scorer.replace(index, paths[index].clone());
    }
    if trace {
        eprintln!("EVAL {evaluated}");
    }
}

pub(in crate::layout) fn simplify_path(
    path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    simplify_path_with_clearances(path, obstacles, scorer, [Terminal::plain(16); 2])
}

pub(in crate::layout) fn simplify_path_with_clearances(
    path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
    terminals: [Terminal; 2],
) -> Vec<Point> {
    select_shortcuts(path, obstacles, scorer, terminals, 1, &mut 0, false)
}

fn select_shortcuts(
    mut path: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
    terminals: [Terminal; 2],
    budget: usize,
    evaluated: &mut usize,
    trace: bool,
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
        let mut window = selection::Window::new(budget);
        let mut first = None;
        'scan: for from in 0..path.len().saturating_sub(2) {
            for to in (from + 2..path.len()).rev() {
                for middle in simple::candidates(path[from], path[to]) {
                    *evaluated += 1;
                    let candidate = simplify(
                        path[..from]
                            .iter()
                            .copied()
                            .chain(middle)
                            .chain(path[to + 1..].iter().copied()),
                        Simplification::PreserveReversals,
                    );
                    let eligible =
                        ports::valid_with_clearances(&candidate, start, end, obstacles, terminals)
                            && candidate.windows(3).all(|p| {
                                !((p[0].x == p[1].x && p[1].x == p[2].x)
                                    || (p[0].y == p[1].y && p[1].y == p[2].y))
                            });
                    let scored = if eligible {
                        scorer
                            .shortcut_cost(&path, &candidate, obstacles)
                            .map(|cost| (cost, candidate))
                    } else {
                        None
                    };
                    if trace && first.is_none() {
                        first = scored.clone();
                    }
                    if window.consider(scored) {
                        break 'scan;
                    }
                }
            }
        }
        if trace {
            if let (Some((first_cost, first)), Some((best_cost, best))) = (first, &window.best) {
                if &first != best {
                    eprintln!("CHOICE {path:?} | {first:?} {first_cost} | {best:?} {best_cost:?}");
                }
            }
        }
        match window.best {
            Some((_, next)) => path = next,
            None => return path,
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/shortcuts.rs"]
mod tests;
