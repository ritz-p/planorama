use super::coverage::Coverage;
use crate::layout::{Bounds, Point};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/scoring.rs"]
mod tests;

pub(in crate::layout) const CROSSING_COST: u128 = 512;
pub(in crate::layout) const BEND_COST: u128 = 96;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::layout) struct Score {
    node_crossings: usize,
    cost: u128,
    overlap: u128,
    crossings: usize,
    bends: usize,
    length: u128,
    upper_detours: usize,
}

#[derive(Default)]
pub(in crate::layout) struct Scorer {
    horizontal: BTreeMap<usize, Coverage>,
    vertical: BTreeMap<usize, Coverage>,
    horizontal_paths: BTreeMap<usize, Vec<(usize, usize, usize)>>,
    vertical_paths: BTreeMap<usize, Vec<(usize, usize, usize)>>,
    paths: Vec<Vec<Point>>,
    peers: Vec<Bounds>,
    soft: Vec<Bounds>,
}

impl Scorer {
    pub(in crate::layout) fn choose(
        &self,
        candidates: Vec<Vec<Point>>,
        obstacles: &[Bounds],
    ) -> Option<Vec<Point>> {
        let reference = candidates.iter().min_by_key(|p| {
            let s = self.score(p, obstacles);
            (s.node_crossings, s.length + s.bends as u128 * BEND_COST)
        })?;
        let baseline = self.score(reference, obstacles);
        candidates
            .into_iter()
            .filter(|p| {
                let s = self.score(p, obstacles);
                s.node_crossings <= baseline.node_crossings
                    && s.bends <= baseline.bends + 6
                    && s.length <= baseline.length * 2 + 512
            })
            .min_by_key(|p| self.score(p, obstacles))
    }
    pub(in crate::layout) fn set_soft(&mut self, soft: Vec<Bounds>) {
        self.soft = soft;
    }

    pub(in crate::layout) fn soft(&self) -> &[Bounds] {
        &self.soft
    }

    fn occlusion(&self, a: Point, b: Point) -> u128 {
        Self::occlusion_in(&self.soft, a, b)
    }

    fn occlusion_in(soft: &[Bounds], a: Point, b: Point) -> u128 {
        soft.iter()
            .filter(|&&bounds| super::crosses(a, b, bounds))
            .map(|bounds| {
                if a.y == b.y {
                    a.x.max(b.x)
                        .min(bounds.right())
                        .saturating_sub(a.x.min(b.x).max(bounds.origin.x))
                        as u128
                } else {
                    a.y.max(b.y)
                        .min(bounds.origin.y + bounds.height)
                        .saturating_sub(a.y.min(b.y).max(bounds.origin.y))
                        as u128
                }
            })
            .sum::<u128>()
            * 2
    }
    pub(in crate::layout) fn len(&self) -> usize {
        self.paths.len()
    }

    pub(in crate::layout) fn truncate(&mut self, len: usize) {
        for index in (len..self.paths.len()).rev() {
            self.remove(index);
        }
        self.paths.truncate(len);
    }

    pub(in crate::layout) fn safe_shortcut(
        &self,
        old: &[Point],
        new: &[Point],
        obstacles: &[Bounds],
    ) -> bool {
        let length = |path: &[Point]| {
            path.windows(2)
                .map(|p| p[0].x.abs_diff(p[1].x) as u128 + p[0].y.abs_diff(p[1].y) as u128)
                .sum::<u128>()
        };
        if new.len() > old.len()
            || length(new) > length(old)
            || (new.len() == old.len() && length(new) == length(old))
        {
            return false;
        }
        let a = self.score(old, obstacles);
        let b = self.score(new, obstacles);
        b.node_crossings == 0
            && b.cost < a.cost
            && b.bends <= a.bends
            && b.length <= a.length
            && (b.bends < a.bends || b.length < a.length)
    }

    pub(in crate::layout) fn set_peers(&mut self, peers: Vec<Bounds>) {
        self.peers = peers;
    }

    pub(in crate::layout) fn peers(&self) -> &[Bounds] {
        &self.peers
    }
    pub(in crate::layout) fn uses_endpoint(&self, point: Point) -> bool {
        self.paths
            .iter()
            .any(|path| path.first() == Some(&point) || path.last() == Some(&point))
    }
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
                * CROSSING_COST
        })
    }

    pub(in crate::layout) fn readability_cost(&self, points: &[Point]) -> u128 {
        self.score(points, &[]).cost
    }

    pub(in crate::layout) fn readability_cost_with_soft(
        &self,
        points: &[Point],
        soft: &[Bounds],
    ) -> u128 {
        self.readability_cost(points)
            - points
                .windows(2)
                .map(|p| self.occlusion(p[0], p[1]))
                .sum::<u128>()
            + points
                .windows(2)
                .map(|p| Self::occlusion_in(soft, p[0], p[1]))
                .sum::<u128>()
    }

    pub(in crate::layout) fn score(&self, points: &[Point], nodes: &[Bounds]) -> Score {
        let mut score = Score {
            node_crossings: 0,
            cost: 0,
            overlap: 0,
            crossings: 0,
            bends: points.len().saturating_sub(2),
            length: 0,
            upper_detours: self
                .peers
                .iter()
                .filter(|b| {
                    points.windows(2).any(|p| {
                        p[0].y == p[1].y
                            && p[0].y <= b.origin.y
                            && p[0].x.min(p[1].x) < b.right()
                            && p[0].x.max(p[1].x) > b.origin.x
                    })
                })
                .count(),
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
            score.cost += self.occlusion(a, b);
            score.node_crossings += nodes
                .iter()
                .filter(|&&node| super::crosses(a, b, node))
                .count();
        }
        let mut crossings = BTreeSet::new();
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let horizontal = a.y == b.y;
            let (index, coordinate, from, to) = if horizontal {
                (&self.vertical_paths, a.y, a.x, b.x)
            } else {
                (&self.horizontal_paths, a.x, a.y, b.y)
            };
            if from == to {
                continue;
            }
            for (&along, segments) in index.range((
                std::ops::Bound::Excluded(from.min(to)),
                std::ops::Bound::Excluded(from.max(to)),
            )) {
                for &(path, start, end) in segments {
                    if start < coordinate && coordinate < end {
                        let (x, y) = if horizontal {
                            (along, coordinate)
                        } else {
                            (coordinate, along)
                        };
                        crossings.insert((path, x, y));
                    }
                }
            }
        }
        score.crossings = crossings.len();
        score.cost += score.overlap * 8
            + score.crossings as u128 * CROSSING_COST
            + score.bends as u128 * BEND_COST
            + score.length;
        score
    }

    pub(in crate::layout) fn insert(&mut self, points: Vec<Point>) {
        let index = self.paths.len();
        self.paths.push(Vec::new());
        self.replace(index, points);
    }

    pub(in crate::layout) fn remove(&mut self, index: usize) {
        let points = std::mem::take(&mut self.paths[index]);
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (coverage, segments, coordinate, from, to) = if a.y == b.y {
                (
                    &mut self.horizontal,
                    &mut self.horizontal_paths,
                    a.y,
                    a.x,
                    b.x,
                )
            } else {
                (&mut self.vertical, &mut self.vertical_paths, a.x, a.y, b.y)
            };
            if from == to {
                continue;
            }
            coverage.get_mut(&coordinate).unwrap().remove(from, to);
            if coverage[&coordinate].is_empty() {
                coverage.remove(&coordinate);
            }
            if let Some(entries) = segments.get_mut(&coordinate) {
                entries.retain(|&(path, _, _)| path != index);
                if entries.is_empty() {
                    segments.remove(&coordinate);
                }
            }
        }
    }

    pub(in crate::layout) fn replace(&mut self, path_index: usize, points: Vec<Point>) {
        self.remove(path_index);
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
                index
                    .entry(coordinate)
                    .or_default()
                    .push((path_index, from.min(to), from.max(to)));
            }
        }
        self.paths[path_index] = points;
    }

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
        overlap * 8 + crossings as u128 * CROSSING_COST + self.occlusion(a, b)
    }
}

#[cfg(test)]
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
