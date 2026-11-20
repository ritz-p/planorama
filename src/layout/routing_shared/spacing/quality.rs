use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::layout) struct Shape {
    pub length: usize,
    pub bends: usize,
    pub offsets: usize,
    pub reversals: usize,
    pub backtracking: usize,
    pub s_jogs: usize,
}

impl Shape {
    pub(in crate::layout) fn measure(path: &[Point]) -> Self {
        let mut result = Self {
            length: length(path),
            bends: path.len().saturating_sub(2),
            ..Self::default()
        };
        let mut directions = [None; 2];
        let mut travel = [[0; 2]; 2];
        for p in path.windows(2) {
            let axis = usize::from(p[0].x == p[1].x);
            let (a, b) = if axis == 0 {
                (p[0].x, p[1].x)
            } else {
                (p[0].y, p[1].y)
            };
            if a == b {
                continue;
            }
            let direction = a < b;
            if directions[axis].is_some_and(|previous| previous != direction) {
                result.reversals += 1;
            }
            directions[axis] = Some(direction);
            travel[axis][usize::from(direction)] += a.abs_diff(b);
        }
        result.backtracking = travel.iter().map(|d| d[0].min(d[1])).sum();
        let mut offsets = Vec::new();
        for i in 1..path.len().saturating_sub(2) {
            let a = path[i - 1];
            let b = path[i];
            let c = path[i + 1];
            let d = path[i + 2];
            let horizontal = b.y == c.y;
            let (before, after) = if horizontal {
                (b.y.cmp(&a.y), d.y.cmp(&c.y))
            } else {
                (b.x.cmp(&a.x), d.x.cmp(&c.x))
            };
            let short = length(&[b, c]);
            let legs = length(&[a, b]).min(length(&[c, d]));
            if before == after && short * 2 < legs && short < PREFERRED * 2 {
                result.offsets += 1;
                offsets.push((
                    i,
                    horizontal,
                    if horizontal { c.x > b.x } else { c.y > b.y },
                ));
            }
        }
        for pair in offsets.windows(2) {
            if pair[1].0 == pair[0].0 + 2 && pair[1].1 == pair[0].1 && pair[1].2 != pair[0].2 {
                result.s_jogs += 1;
            }
        }
        result
    }

    pub(super) fn fits(self, base: Self) -> bool {
        self.bends <= base.bends
            && self.offsets <= base.offsets
            && self.reversals <= base.reversals
            && self.backtracking <= base.backtracking
            && self.s_jogs <= base.s_jogs
            && self.length <= base.length + 128
    }
}

pub(super) fn improves(
    graph: &Graph,
    before: &[Vec<Point>],
    after: &[Vec<Point>],
    before_borders: &[super::super::borders::Metrics],
    after_borders: &[super::super::borders::Metrics],
    expanded: bool,
) -> bool {
    if before_borders.iter().zip(after_borders).any(|(a, b)| {
        b.minimum_deficit > a.minimum_deficit
            || b.minimum.unwrap_or(i64::MAX)
                < a.minimum
                    .unwrap_or(i64::MAX)
                    .min((super::super::borders::MINIMUM * 4) as i64)
    }) {
        return false;
    }
    let clarity = |paths: &[Vec<Point>]| {
        paths.iter().fold((0, 0, 0, 0), |a, p| {
            let s = Shape::measure(p);
            (
                a.0 + s.s_jogs,
                a.1 + s.offsets,
                a.2 + s.reversals,
                a.3 + s.bends,
            )
        })
    };
    let a = clarity(before);
    let b = clarity(after);
    let excess = |path: &Vec<Point>| {
        let direct = path
            .first()
            .zip(path.last())
            .map_or(0, |(a, b)| a.x.abs_diff(b.x) + a.y.abs_diff(b.y));
        length(path).saturating_sub(direct)
    };
    let length_before: usize = before.iter().map(excess).sum();
    let length_after: usize = after.iter().map(excess).sum();
    let proximity_before = measure(graph, before);
    let proximity_after = measure(graph, after);
    let cost = |proximity: &Metrics, length: usize, borders: &[super::super::borders::Metrics]| {
        (proximity.deficit / 4) as u128
            + length as u128
            + borders.iter().map(|m| m.cost()).sum::<u128>()
    };
    b < a
        || (b == a
            && cost(&proximity_after, length_after, after_borders) + u128::from(expanded) * 32
                < cost(&proximity_before, length_before, before_borders))
}

pub(super) fn terminal_lane_slack(graph: &Graph, paths: &[Vec<Point>]) -> Vec<usize> {
    let mut slack = vec![0; paths.len()];
    let stroke = |s: Segment| crate::view::geometry::relationship_stroke(graph.edges[s.edge].kind);
    for (a, b, _) in pairs(graph, paths, PREFERRED - 1) {
        let distance = a.coordinate.abs_diff(b.coordinate);
        if distance * 8 > stroke(a) + stroke(b) {
            continue;
        }
        let edge_a = &graph.edges[a.edge];
        let edge_b = &graph.edges[b.edge];
        let path_a = &paths[a.edge];
        let path_b = &paths[b.edge];
        let distinct_ports = |end| {
            let port_a = terminal(path_a, end);
            let port_b = terminal(path_b, end);
            port_a.side == port_b.side && port_a.point != port_b.point
        };
        let at_source = edge_a.from == edge_b.from && a.index == 1 && b.index == 1;
        let at_target =
            edge_a.to == edge_b.to && a.index + 3 == path_a.len() && b.index + 3 == path_b.len();
        if (at_source && distinct_ports(0)) || (at_target && distinct_ports(1)) {
            for edge in [a.edge, b.edge] {
                slack[edge] = slack[edge].max(PREFERRED - distance);
            }
        }
    }
    slack
}

pub(super) fn translated_baseline(
    original: &[Shape],
    reference: &[Vec<Point>],
    slack: &[usize],
) -> Vec<Shape> {
    original
        .iter()
        .zip(reference)
        .zip(slack)
        .map(|((&original, path), slack)| {
            let geometry = Shape::measure(path);
            Shape {
                length: geometry.length,
                backtracking: geometry.backtracking + slack,
                ..original
            }
        })
        .collect()
}
