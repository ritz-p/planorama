use super::{Bounds, Point, Scorer};
mod obstacles;
use crate::layout::Side;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[cfg(test)]
#[path = "../../../tests/unit/layout/containers/search.rs"]
mod tests;

pub(in crate::layout) fn find_path(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    search(
        start,
        end,
        obstacles,
        scorer,
        Some((Side::Right, Side::Right)),
    )
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
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for bounds in obstacles
        .iter()
        .chain(scorer.into_iter().flat_map(|s| s.soft()))
    {
        for clearance in if scorer.is_some() {
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
    let blocked = obstacles::Grid::new(&xs, &ys, obstacles);
    let index =
        |p: Point| ys.binary_search(&p.y).unwrap() * xs.len() + xs.binary_search(&p.x).unwrap();
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    let (first, last) = (index(start), index(end));
    let mut distances = vec![u128::MAX; xs.len() * ys.len() * 3];
    let mut previous = vec![None; distances.len()];
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
            let penalty = scorer.map_or(0, |s| {
                s.segment_cost(a, b)
                    + if direction == next_direction {
                        s.junction_cost(a, next_direction == 1)
                    } else {
                        0
                    }
                    + if next == last && next_direction == final_direction {
                        s.junction_cost(b, final_direction == 1)
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
                queue.push(Reverse((candidate, next_state)));
            }
        }
    }
    let mut state = final_state?;
    let mut path = vec![end];
    while state != initial {
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
