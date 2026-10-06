use super::{Bounds, Point, Scorer, crosses};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[cfg(test)]
#[path = "../../../../tests/unit/layout/containers/search.rs"]
mod tests;

pub(super) fn find_path(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for bounds in obstacles {
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
    let index =
        |p: Point| ys.binary_search(&p.y).unwrap() * xs.len() + xs.binary_search(&p.x).unwrap();
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    let (first, last) = (index(start), index(end));
    // Retain arrival direction so bend penalties preserve optimal substructure.
    let mut distances = vec![u128::MAX; xs.len() * ys.len() * 3];
    let mut previous = vec![None; distances.len()];
    // The source stub is horizontal, so an initial vertical step is a bend.
    let initial = first * 3 + 1;
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
            // The caller attaches a rightward source stub and a leftward
            // target stub. Do not reverse over either fixed segment.
            if (current == first && b.x < a.x) || (next == last && a.x < b.x) {
                continue;
            }
            if obstacles.iter().any(|&bounds| crosses(a, b, bounds)) {
                continue;
            }
            let next_direction = if a.y == b.y { 1 } else { 2 };
            let penalty = scorer.map_or(0, |s| {
                s.segment_cost(a, b)
                    + if current == first && next_direction == 1 {
                        s.horizontal_junction_cost(a)
                    } else {
                        0
                    }
                    + if next == last && next_direction == 1 {
                        s.horizontal_junction_cost(b)
                    } else {
                        0
                    }
                    + if direction != next_direction {
                        24
                    } else {
                        0
                    }
                    // Include the horizontal target stub before queueing the
                    // terminal state; otherwise early exit can pick a worse
                    // arrival direction even when a cheaper one is pending.
                    + if next == last && next_direction != 1 { 24 } else { 0 }
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
    let mut state = final_state.expect("nested layout leaves routing corridors around every card");
    let mut path = vec![end];
    while state != initial {
        state = previous[state].expect("reachable grid state has a predecessor");
        path.push(point(state / 3));
    }
    path.reverse();
    let mut compact: Vec<Point> = Vec::new();
    for point in path {
        if compact.len() >= 2 {
            let (a, b) = (compact[compact.len() - 2], compact[compact.len() - 1]);
            if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) {
                compact.pop();
            }
        }
        compact.push(point);
    }
    compact
}
