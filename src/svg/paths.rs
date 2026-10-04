use crate::layout::Point;
use std::cmp::Ordering;
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/paths.rs"]
mod tests;

const CORNER_RADIUS: f64 = 8.0;

pub(super) fn rounded(points: &[Point]) -> String {
    let mut points = points.to_vec();
    points.dedup();
    let mut path = String::new();
    let first = match points.first() {
        Some(first) => first,
        None => return path,
    };
    write!(path, "M {} {} ", first.x, first.y).unwrap();
    for window in points.windows(3) {
        let [before, corner, after] = [window[0], window[1], window[2]];
        let turns = (before.x == corner.x && corner.y == after.y)
            || (before.y == corner.y && corner.x == after.x);
        match turns {
            true => {
                let incoming = before.x.abs_diff(corner.x) + before.y.abs_diff(corner.y);
                let outgoing = after.x.abs_diff(corner.x) + after.y.abs_diff(corner.y);
                let radius = CORNER_RADIUS
                    .min(incoming as f64 / 2.0)
                    .min(outgoing as f64 / 2.0);
                let (enter_x, enter_y) = toward(corner, before, radius);
                let (exit_x, exit_y) = toward(corner, after, radius);
                write!(
                    path,
                    "L {enter_x} {enter_y} Q {} {} {exit_x} {exit_y} ",
                    corner.x, corner.y
                )
                .unwrap();
            }
            false => write!(path, "L {} {} ", corner.x, corner.y).unwrap(),
        }
    }
    if points.len() > 1 {
        let last = points.last().unwrap();
        write!(path, "L {} {} ", last.x, last.y).unwrap();
    }
    path
}

fn toward(from: Point, to: Point, distance: f64) -> (f64, f64) {
    let offset = |from: usize, to: usize| match from.cmp(&to) {
        Ordering::Less => from as f64 + distance,
        Ordering::Greater => from as f64 - distance,
        Ordering::Equal => from as f64,
    };
    (offset(from.x, to.x), offset(from.y, to.y))
}
