use super::{Bounds, Point};
mod coverage;
pub(super) mod ports;
pub(super) mod refinement;
pub(super) mod scoring;
pub(super) mod search;
pub(super) mod shortcuts;
mod simple;
pub(super) mod slots;
pub(super) mod spacing;
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

#[cfg(test)]
#[path = "../../tests/unit/layout/routing_shared.rs"]
mod tests;
