use super::{Bounds, Point, Scorer};
mod costs;
mod obstacles;
mod occlusion;
use crate::layout::Side;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[cfg(test)]
std::thread_local! {
    pub(in crate::layout) static GRID_SEARCH_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(in crate::layout) static GRID_POINT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/containers/search.rs"]
mod tests;

#[cfg(test)]
pub(in crate::layout) fn find_path(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    crate::work::complete(find_path_step(start, end, obstacles, scorer))
}

pub(in crate::layout) async fn find_path_step(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    crate::work::checkpoint().await;
    search_step(
        start,
        end,
        obstacles,
        scorer,
        Some((Side::Right, Side::Right)),
    )
    .await
    .expect("nested layout leaves routing corridors around every card")
}

pub(in crate::layout) fn to_margin(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    search(start, end, obstacles, None, None).expect("state margin is reachable")
}

pub(super) fn search(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    ports: Option<(Side, Side)>,
) -> Option<Vec<Point>> {
    crate::work::complete(search_step(start, end, obstacles, scorer, ports))
}

pub(super) async fn search_step(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    ports: Option<(Side, Side)>,
) -> Option<Vec<Point>> {
    crate::work::checkpoint().await;
    #[cfg(test)]
    GRID_SEARCH_COUNT.set(GRID_SEARCH_COUNT.get() + 1);
    search_grid_step(start, end, obstacles, scorer, ports, false).await
}

#[cfg(test)]
pub(in crate::layout) fn shortest(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    ports: (Side, Side),
) -> Option<Vec<Point>> {
    search_grid(start, end, obstacles, None, Some(ports), true)
}

#[cfg(test)]
fn search_grid(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    ports: Option<(Side, Side)>,
    exact: bool,
) -> Option<Vec<Point>> {
    crate::work::complete(search_grid_step(
        start, end, obstacles, scorer, ports, exact,
    ))
}

async fn search_grid_step(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    ports: Option<(Side, Side)>,
    exact: bool,
) -> Option<Vec<Point>> {
    crate::work::checkpoint().await;
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for &(horizontal, coordinate) in scorer.into_iter().flat_map(|s| &s.routing_axes) {
        crate::work::checkpoint().await;
        if horizontal {
            ys.push(coordinate);
        } else {
            xs.push(coordinate);
        }
    }
    for (horizontal, coordinate) in scorer.into_iter().flat_map(|s| s.border_axes(start, end)) {
        crate::work::checkpoint().await;
        if horizontal {
            ys.push(coordinate);
        } else {
            xs.push(coordinate);
        }
    }
    for bounds in obstacles
        .iter()
        .chain(scorer.into_iter().flat_map(|s| s.soft()).map(|o| &o.bounds))
    {
        if exact {
            xs.extend([bounds.origin.x, bounds.right()]);
            ys.extend([bounds.origin.y, bounds.origin.y + bounds.height]);
        }
        for clearance in if scorer.is_some() {
            crate::work::checkpoint().await;
            &[8, 16, 24][..]
        } else {
            &[16][..]
        } {
            xs.extend([
                bounds.origin.x.saturating_sub(*clearance),
                bounds.right() + clearance,
            ]);
            ys.extend([
                bounds.origin.y.saturating_sub(*clearance),
                bounds.origin.y + bounds.height + clearance,
            ]);
        }
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    crate::work::check_limit(xs.len().saturating_mul(ys.len()), 131_072).await;
    crate::work::check_memory(
        (xs.len() as u64)
            .saturating_mul(ys.len() as u64)
            .saturating_mul(256),
    )
    .await;
    crate::work::checkpoint().await;
    #[cfg(test)]
    GRID_POINT_COUNT.set(xs.len() * ys.len());
    let blocked = obstacles::Grid::new_step(&xs, &ys, obstacles).await;
    crate::work::checkpoint().await;
    let mut costs = if let Some(scorer) = scorer {
        Some(costs::Grid::new_step(&xs, &ys, scorer).await)
    } else {
        None
    };
    let index =
        |p: Point| ys.binary_search(&p.y).unwrap() * xs.len() + xs.binary_search(&p.x).unwrap();
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    let (first, last) = (index(start), index(end));
    let mut distances = crate::work::filled_vec(u128::MAX, xs.len() * ys.len() * 3).await;
    let mut previous = crate::work::filled_vec(None, distances.len()).await;
    let direction = |side| match side {
        Side::Left | Side::Right => 1,
        Side::Top | Side::Bottom => 2,
    };
    let (initial_direction, final_direction) =
        ports.map_or((1, 1), |(a, b)| (direction(a), direction(b)));
    let initial = first * 3 + initial_direction;
    let mut queue = BinaryHeap::from([Reverse((0, initial))]);
    distances[initial] = 0;
    let mut final_state = None;
    while let Some(Reverse((distance, state))) = queue.pop() {
        crate::work::checkpoint().await;
        let (current, direction) = (state / 3, state % 3);
        if distance != distances[state] {
            continue;
        }
        if current == last {
            final_state = Some(state);
            break;
        }
        let (x, y) = (current % xs.len(), current / xs.len());
        let neighbors = [
            x.checked_sub(1).map(|x| y * xs.len() + x),
            (x + 1 < xs.len()).then_some(current + 1),
            y.checked_sub(1).map(|y| y * xs.len() + x),
            (y + 1 < ys.len()).then_some(current + xs.len()),
        ];
        for next in neighbors.into_iter().flatten() {
            crate::work::checkpoint().await;
            let (a, b) = (point(current), point(next));
            if ports.is_some_and(|(source, target)| {
                (current == first && reverses(source, a, b))
                    || (next == last && reverses(target, b, a))
            }) {
                continue;
            }
            if blocked.blocks(current, next) {
                continue;
            }
            let next_direction = if a.y == b.y { 1 } else { 2 };
            let penalty = costs.as_mut().map_or(0, |costs| {
                costs.segment(current, next, a, b)
                    + if direction == next_direction {
                        costs.junction(current, a, next_direction == 1)
                    } else {
                        0
                    }
                    + if next == last && next_direction == final_direction {
                        costs.junction(next, b, final_direction == 1)
                    } else {
                        0
                    }
                    + if direction != next_direction {
                        super::scoring::BEND_COST
                    } else {
                        0
                    }
                    + if next == last && next_direction != final_direction {
                        super::scoring::BEND_COST
                    } else {
                        0
                    }
            });
            let candidate =
                distance + a.x.abs_diff(b.x) as u128 + a.y.abs_diff(b.y) as u128 + penalty;
            let next_state = next * 3 + next_direction;
            if candidate < distances[next_state] {
                distances[next_state] = candidate;
                previous[next_state] = Some(state);
                crate::work::check_limit(queue.len().saturating_add(1), 262_144).await;
                if queue.len() == queue.capacity() {
                    crate::work::check_memory(
                        (queue.capacity() as u64)
                            .saturating_mul(64)
                            .saturating_add(256),
                    )
                    .await;
                    if crate::work::limited() {
                        crate::work::check_allocation(queue.try_reserve(1).is_ok()).await;
                    }
                }
                queue.push(Reverse((candidate, next_state)));
            }
        }
    }
    let mut state = final_state?;
    let mut path = vec![end];
    while state != initial {
        crate::work::checkpoint().await;
        state = previous[state].expect("reachable grid state has a predecessor");
        path.push(point(state / 3));
    }
    path.reverse();
    Some(super::simplify(
        path,
        super::Simplification::CollapseCollinear,
    ))
}

fn reverses(side: Side, a: Point, b: Point) -> bool {
    match side {
        Side::Right => b.x < a.x,
        Side::Left => b.x > a.x,
        Side::Bottom => b.y < a.y,
        Side::Top => b.y > a.y,
    }
}
