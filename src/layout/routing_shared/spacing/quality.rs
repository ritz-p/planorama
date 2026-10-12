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
    expanded: bool,
) -> bool {
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
    let length_before: usize = before.iter().map(|p| length(p)).sum();
    let length_after: usize = after.iter().map(|p| length(p)).sum();
    let proximity_before = measure(graph, before);
    let proximity_after = measure(graph, after);
    b < a
        || (b == a
            && proximity_after.deficit / 4 + length_after + usize::from(expanded) * 32
                < proximity_before.deficit / 4 + length_before)
}
