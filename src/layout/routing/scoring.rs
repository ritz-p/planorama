use super::coverage::Coverage;
use crate::layout::{Bounds, Point};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/scoring.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::layout) struct Score {
    node_crossings: usize,
    overlap: u128,
    crossings: usize,
    bends: usize,
    length: u128,
}

#[derive(Default)]
pub(in crate::layout) struct Scorer {
    horizontal: BTreeMap<usize, Coverage>,
    vertical: BTreeMap<usize, Coverage>,
    horizontal_paths: BTreeMap<usize, Vec<(usize, usize, usize)>>,
    vertical_paths: BTreeMap<usize, Vec<(usize, usize, usize)>>,
    paths: Vec<Vec<Point>>,
}

impl Scorer {
    pub(in crate::layout) fn overlaps(&self, points: &[Point]) -> bool {
        points.windows(2).any(|pair| {
            let (a, b) = (pair[0], pair[1]);
            let (index, coordinate, from, to) = if a.y == b.y {
                (&self.horizontal, a.y, a.x, b.x)
            } else {
                (&self.vertical, a.x, a.y, b.y)
            };
            index
                .get(&coordinate)
                .is_some_and(|coverage| coverage.overlap(from, to) > 0)
        })
    }

    /// Crossings at a grid vertex, charged only for straight continuation.
    /// Segment costs exclude both endpoints until the next direction is known.
    pub(in crate::layout) fn junction_cost(&self, point: Point, horizontal: bool) -> u128 {
        let (index, coordinate, along) = if horizontal {
            (&self.vertical_paths, point.x, point.y)
        } else {
            (&self.horizontal_paths, point.y, point.x)
        };
        index.get(&coordinate).map_or(0, |segments| {
            segments
                .iter()
                .filter(|&&(_, from, to)| from < along && along < to)
                .map(|&(path, _, _)| path)
                .collect::<BTreeSet<_>>()
                .len() as u128
                * 2048
        })
    }

    pub(in crate::layout) fn readability_cost(&self, points: &[Point]) -> u128 {
        let score = self.score(points, &[]);
        score.overlap * 8 + score.crossings as u128 * 2048 + score.bends as u128 * 24 + score.length
    }

    pub(in crate::layout) fn score(&self, points: &[Point], nodes: &[Bounds]) -> Score {
        let mut score = Score {
            node_crossings: 0,
            overlap: 0,
            crossings: 0,
            bends: points.len().saturating_sub(2),
            length: 0,
        };
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let horizontal = a.y == b.y;
            let (index, coordinate, from, to) = match horizontal {
                true => (&self.horizontal, a.y, a.x, b.x),
                false => (&self.vertical, a.x, a.y, b.y),
            };
            score.overlap += index.get(&coordinate).map_or(0, |c| c.overlap(from, to));
            score.length += a.x.abs_diff(b.x) as u128 + a.y.abs_diff(b.y) as u128;
            score.node_crossings += nodes
                .iter()
                .filter(|node| match horizontal {
                    true => {
                        a.y > node.origin.y
                            && a.y < node.origin.y + node.height
                            && a.x.max(b.x) > node.origin.x
                            && a.x.min(b.x) < node.right()
                    }
                    false => {
                        a.x > node.origin.x
                            && a.x < node.right()
                            && a.y.max(b.y) > node.origin.y
                            && a.y.min(b.y) < node.origin.y + node.height
                    }
                })
                .count();
        }
        for path in &self.paths {
            let mut crossings = BTreeSet::new();
            for first in points.windows(2) {
                for second in path.windows(2) {
                    if let Some(point) = crossing(first, second) {
                        crossings.insert(point);
                    }
                }
            }
            score.crossings += crossings.len();
        }
        score
    }

    pub(in crate::layout) fn insert(&mut self, points: Vec<Point>) {
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            match a.y == b.y {
                true => self.horizontal.entry(a.y).or_default().insert(a.x, b.x),
                false => self.vertical.entry(a.x).or_default().insert(a.y, b.y),
            }
            let (index, coordinate, from, to) = if a.y == b.y {
                (&mut self.horizontal_paths, a.y, a.x, b.x)
            } else {
                (&mut self.vertical_paths, a.x, a.y, b.y)
            };
            if from != to {
                index.entry(coordinate).or_default().push((
                    self.paths.len(),
                    from.min(to),
                    from.max(to),
                ));
            }
        }
        self.paths.push(points);
    }

    /// Search cost for a grid segment, excluding endpoint crossings.
    /// The search charges those separately when it knows the next direction.
    pub(in crate::layout) fn segment_cost(&self, a: Point, b: Point) -> u128 {
        let horizontal = a.y == b.y;
        let (parallel, perpendicular, coordinate, from, to) = if horizontal {
            (&self.horizontal, &self.vertical_paths, a.y, a.x, b.x)
        } else {
            (&self.vertical, &self.horizontal_paths, a.x, a.y, b.y)
        };
        let overlap = parallel.get(&coordinate).map_or(0, |c| c.overlap(from, to));
        let crossings = perpendicular
            .range((
                std::ops::Bound::Excluded(from.min(to)),
                std::ops::Bound::Excluded(from.max(to)),
            ))
            .map(|(_, segments)| {
                segments
                    .iter()
                    .filter(|&&(_, from, to)| from < coordinate && coordinate < to)
                    .map(|&(path, _, _)| path)
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .sum::<usize>();
        overlap * 8 + crossings as u128 * 2048
    }
}

fn crossing(first: &[Point], second: &[Point]) -> Option<(usize, usize)> {
    let (h, v) = match (first[0].y == first[1].y, second[0].y == second[1].y) {
        (true, false) => (first, second),
        (false, true) => (second, first),
        _ => return None,
    };
    let (x, y) = (v[0].x, h[0].y);
    match x > h[0].x.min(h[1].x)
        && x < h[0].x.max(h[1].x)
        && y > v[0].y.min(v[1].y)
        && y < v[0].y.max(v[1].y)
    {
        true => Some((x, y)),
        false => None,
    }
}
