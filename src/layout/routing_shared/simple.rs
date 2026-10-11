use super::{Point, Simplification, simplify};

pub(super) fn candidates(start: Point, end: Point) -> Vec<Vec<Point>> {
    let x = start.x.min(end.x) + start.x.abs_diff(end.x) / 2;
    let y = start.y.min(end.y) + start.y.abs_diff(end.y) / 2;
    [
        vec![start, end],
        vec![
            start,
            Point {
                x: end.x,
                y: start.y,
            },
            end,
        ],
        vec![
            start,
            Point {
                x: start.x,
                y: end.y,
            },
            end,
        ],
        vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end],
        vec![start, Point { x: start.x, y }, Point { x: end.x, y }, end],
    ]
    .into_iter()
    .map(|path| simplify(path, Simplification::PreserveReversals))
    .collect()
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/simple.rs"]
mod tests;
