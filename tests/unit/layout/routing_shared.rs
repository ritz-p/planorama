use super::*;

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
