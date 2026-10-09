//! Shared routing geometry, occupancy/scoring and orthogonal search primitives.
use super::{Bounds, Point};
mod coverage;
pub(super) mod ports;
pub(super) mod scoring;
pub(super) mod search;
use scoring::Scorer;

pub(super) enum Simplification {
    CollapseCollinear,
    PreserveReversals,
}

pub(super) fn simplify(
    points: impl IntoIterator<Item = Point>,
    policy: Simplification,
) -> Vec<Point> {
    let mut result: Vec<Point> = Vec::new();
    for point in points {
        if result.last() == Some(&point) {
            continue;
        }
        while let [.., a, b] = result.as_slice() {
            let aligned = (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y);
            let forward = a.x.abs_diff(b.x) + b.x.abs_diff(point.x) == a.x.abs_diff(point.x)
                && a.y.abs_diff(b.y) + b.y.abs_diff(point.y) == a.y.abs_diff(point.y);
            if aligned && (forward || matches!(policy, Simplification::CollapseCollinear)) {
                result.pop();
            } else {
                break;
            }
        }
        result.push(point);
    }
    result
}

#[test]
fn simplification_preserves_endpoint_reversals_when_requested() {
    let points = [
        Point { x: 0, y: 0 },
        Point { x: 10, y: 0 },
        Point { x: 10, y: 0 },
        Point { x: 5, y: 0 },
        Point { x: 5, y: 10 },
    ];
    assert_eq!(
        simplify(points, Simplification::PreserveReversals),
        vec![points[0], points[1], points[3], points[4]]
    );
    assert_eq!(
        simplify(points, Simplification::CollapseCollinear),
        vec![points[0], points[3], points[4]]
    );
}
pub(super) fn crosses(a: Point, b: Point, bounds: Bounds) -> bool {
    let Point { x, y } = bounds.origin;
    match a.x == b.x {
        true => {
            a.x > x
                && a.x < x + bounds.width
                && a.y.max(b.y) > y
                && a.y.min(b.y) < y + bounds.height
        }
        false => {
            a.y > y
                && a.y < y + bounds.height
                && a.x.max(b.x) > x
                && a.x.min(b.x) < x + bounds.width
        }
    }
}
