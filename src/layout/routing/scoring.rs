use super::coverage::Coverage;
use crate::layout::{NODE_HEIGHT, NODE_WIDTH, Point};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/scoring.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Score {
    node_crossings: usize,
    overlap: u128,
    crossings: usize,
    bends: usize,
    length: u128,
}

#[derive(Default)]
pub(super) struct Scorer {
    horizontal: BTreeMap<usize, Coverage>,
    vertical: BTreeMap<usize, Coverage>,
    paths: Vec<Vec<Point>>,
}

impl Scorer {
    pub(super) fn score(&self, points: &[Point], nodes: &[Point]) -> Score {
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
                        a.y > node.y
                            && a.y < node.y + NODE_HEIGHT
                            && a.x.max(b.x) > node.x
                            && a.x.min(b.x) < node.x + NODE_WIDTH
                    }
                    false => {
                        a.x > node.x
                            && a.x < node.x + NODE_WIDTH
                            && a.y.max(b.y) > node.y
                            && a.y.min(b.y) < node.y + NODE_HEIGHT
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

    pub(super) fn insert(&mut self, points: Vec<Point>) {
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            match a.y == b.y {
                true => self.horizontal.entry(a.y).or_default().insert(a.x, b.x),
                false => self.vertical.entry(a.x).or_default().insert(a.y, b.y),
            }
        }
        self.paths.push(points);
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
