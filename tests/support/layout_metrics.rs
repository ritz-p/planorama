use crate::layout::Point;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../unit/layout/metrics.rs"]
mod tests;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LayoutMetrics {
    pub overlap_distance: u128,
    pub crossing_count: usize,
    pub bend_count: usize,
    pub total_path_length: u128,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EdgeMetrics {
    pub bends: usize,
    pub length: u128,
    pub shortest_valid_length: u128,
    pub excess_distance: u128,
}

impl EdgeMetrics {
    pub fn violation(
        &self,
        from: &str,
        to: &str,
        max_bends: usize,
        max_stretch: u128,
        slack: u128,
    ) -> Option<String> {
        (self.bends > max_bends || self.length > self.shortest_valid_length * max_stretch + slack)
            .then(|| {
                format!(
                    "{from} -> {to}: bends={}, length={}, shortest={}, excess={}, stretch={:.3}",
                    self.bends,
                    self.length,
                    self.shortest_valid_length,
                    self.excess_distance,
                    self.length as f64 / self.shortest_valid_length.max(1) as f64
                )
            })
    }
}

pub(crate) fn measure_edge(path: &[Point], shortest_valid: &[Point]) -> EdgeMetrics {
    assert_eq!(path.first(), shortest_valid.first());
    assert_eq!(path.last(), shortest_valid.last());
    let metrics = measure(&[path.to_vec()]);
    let shortest_valid_length = total_path_length(&segments(&[shortest_valid.to_vec()]));
    assert!(shortest_valid_length <= metrics.total_path_length);
    EdgeMetrics {
        bends: metrics.bend_count,
        length: metrics.total_path_length,
        shortest_valid_length,
        excess_distance: metrics.total_path_length - shortest_valid_length,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct Segment {
    from: Point,
    to: Point,
    axis: Axis,
}

impl Segment {
    fn new(from: Point, to: Point) -> Option<Self> {
        let axis = match (from.x == to.x, from.y == to.y) {
            (true, true) => return None,
            (false, true) => Axis::Horizontal,
            (true, false) => Axis::Vertical,
            (false, false) => panic!("layout metrics require orthogonal paths"),
        };
        Some(Self { from, to, axis })
    }

    fn range(self) -> (usize, usize) {
        match self.axis {
            Axis::Horizontal => (self.from.x.min(self.to.x), self.from.x.max(self.to.x)),
            Axis::Vertical => (self.from.y.min(self.to.y), self.from.y.max(self.to.y)),
        }
    }

    fn length(self) -> u128 {
        let (from, to) = self.range();
        (to - from) as u128
    }

    fn continues(self, next: Self) -> bool {
        self.axis == next.axis
            && self.to == next.from
            && self.to.x.cmp(&self.from.x) == next.to.x.cmp(&next.from.x)
            && self.to.y.cmp(&self.from.y) == next.to.y.cmp(&next.from.y)
    }

    fn overlap(self, other: Self) -> u128 {
        match (self.axis, other.axis) {
            (Axis::Horizontal, Axis::Horizontal) if self.from.y == other.from.y => {}
            (Axis::Vertical, Axis::Vertical) if self.from.x == other.from.x => {}
            _ => return 0,
        }
        let (a, b) = self.range();
        let (c, d) = other.range();
        b.min(d).saturating_sub(a.max(c)) as u128
    }

    fn crossing(self, other: Self) -> Option<(usize, usize)> {
        match (self.axis, other.axis) {
            (Axis::Horizontal, Axis::Vertical) => {
                let (left, right) = self.range();
                let (top, bottom) = other.range();
                let point = (other.from.x, self.from.y);
                (left < point.0 && point.0 < right && top < point.1 && point.1 < bottom)
                    .then_some(point)
            }
            (Axis::Vertical, Axis::Horizontal) => other.crossing(self),
            _ => None,
        }
    }
}

fn segments(paths: &[Vec<Point>]) -> Vec<Vec<Segment>> {
    paths
        .iter()
        .map(|path| {
            let mut segments: Vec<Segment> = Vec::new();
            for segment in path
                .windows(2)
                .filter_map(|pair| Segment::new(pair[0], pair[1]))
            {
                match segments.last_mut() {
                    Some(last) if last.continues(segment) => last.to = segment.to,
                    _ => segments.push(segment),
                }
            }
            segments
        })
        .collect()
}

fn edge_overlap_distance(paths: &[Vec<Segment>]) -> u128 {
    let mut total = 0;
    for (i, path) in paths.iter().enumerate() {
        for other in paths.iter().skip(i + 1) {
            for &a in path {
                for &b in other {
                    total += a.overlap(b);
                }
            }
        }
    }
    total
}

fn edge_crossing_count(paths: &[Vec<Segment>]) -> usize {
    let mut total = 0;
    for (i, path) in paths.iter().enumerate() {
        for other in paths.iter().skip(i + 1) {
            let mut intersections = BTreeSet::new();
            for &a in path {
                for &b in other {
                    intersections.extend(a.crossing(b));
                }
            }
            total += intersections.len();
        }
    }
    total
}

fn bend_count(paths: &[Vec<Segment>]) -> usize {
    paths
        .iter()
        .map(|path| {
            path.windows(2)
                .filter(|pair| pair[0].axis != pair[1].axis)
                .count()
        })
        .sum()
}

fn total_path_length(paths: &[Vec<Segment>]) -> u128 {
    paths.iter().flatten().map(|segment| segment.length()).sum()
}

pub(crate) fn measure(paths: &[Vec<Point>]) -> LayoutMetrics {
    let paths = segments(paths);
    LayoutMetrics {
        overlap_distance: edge_overlap_distance(&paths),
        crossing_count: edge_crossing_count(&paths),
        bend_count: bend_count(&paths),
        total_path_length: total_path_length(&paths),
    }
}
