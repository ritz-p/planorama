use super::lanes::{Lane, VerticalLanes};
use super::simplify;
use crate::layout::Point;

pub(super) struct RouteCandidate {
    pub points: Vec<Point>,
    reservations: Vec<(Lane, usize, usize)>,
}

impl RouteCandidate {
    pub(super) fn direct(start: Point, end: Point, lane: Lane, columns: &[usize]) -> Self {
        let x = columns[lane.gutter] + lane.offset();
        Self {
            points: simplify(vec![
                start,
                Point { x, y: start.y },
                Point { x, y: end.y },
                end,
            ]),
            reservations: vec![(lane, start.y, end.y)],
        }
    }

    pub(super) fn channel(
        start: Point,
        end: Point,
        source: Lane,
        target: Lane,
        y: usize,
        columns: &[usize],
    ) -> Self {
        let left = columns[source.gutter] + source.offset();
        let right = columns[target.gutter] + target.offset();
        Self {
            points: simplify(vec![
                start,
                Point {
                    x: left,
                    y: start.y,
                },
                Point { x: left, y },
                Point { x: right, y },
                Point { x: right, y: end.y },
                end,
            ]),
            reservations: vec![(source, start.y, y), (target, y, end.y)],
        }
    }

    pub(super) fn reserve(&self, lanes: &mut VerticalLanes) {
        for &(lane, from, to) in &self.reservations {
            lanes.reserve(lane, from, to);
        }
    }
}
