use super::{Bounds, Route};
use serde_json::{Value, json};

#[derive(Default, Clone, Copy)]
struct Interval {
    covers: usize,
    length: u64,
}

struct Sweep {
    y: Vec<u64>,
    intervals: Vec<Interval>,
}

impl Sweep {
    fn update(&mut self, node: usize, span: (usize, usize), range: (usize, usize), add: bool) {
        let (left, right) = span;
        if range.0 <= left && right <= range.1 {
            if add {
                self.intervals[node].covers += 1;
            } else {
                self.intervals[node].covers -= 1;
            }
        } else {
            let mid = left + (right - left) / 2;
            if range.0 < mid {
                self.update(node * 2, (left, mid), range, add);
            }
            if mid < range.1 {
                self.update(node * 2 + 1, (mid, right), range, add);
            }
        }
        self.intervals[node].length = if self.intervals[node].covers > 0 {
            self.y[right] - self.y[left]
        } else if right - left == 1 {
            0
        } else {
            self.intervals[node * 2].length + self.intervals[node * 2 + 1].length
        };
    }
}

pub(super) fn measure(width: usize, height: usize, frames: &[Bounds], edges: &[Route]) -> Value {
    let eighth_pixels = |n: usize| (n as u64).saturating_mul(8);
    let mut events = Vec::new();
    let mut y = Vec::new();
    let mut rectangle = |x0: u64, y0: u64, x1: u64, y1: u64| {
        let x1 = x1.min(eighth_pixels(width));
        let y1 = y1.min(eighth_pixels(height));
        if x0 < x1 && y0 < y1 {
            events.push((x0, y0, y1, true));
            events.push((x1, y0, y1, false));
            y.extend([y0, y1]);
        }
    };
    for frame in frames {
        rectangle(
            eighth_pixels(frame.origin.x),
            eighth_pixels(frame.origin.y),
            eighth_pixels(frame.origin.x.saturating_add(frame.width)),
            eighth_pixels(frame.origin.y.saturating_add(frame.height)),
        );
    }
    for edge in edges {
        let radius = edge.stroke as u64;
        for points in edge.path.windows(2) {
            let (a, b) = (points[0], points[1]);
            if a.y == b.y {
                rectangle(
                    eighth_pixels(a.x.min(b.x)),
                    eighth_pixels(a.y).saturating_sub(radius),
                    eighth_pixels(a.x.max(b.x)),
                    eighth_pixels(a.y).saturating_add(radius),
                );
            } else if a.x == b.x {
                rectangle(
                    eighth_pixels(a.x).saturating_sub(radius),
                    eighth_pixels(a.y.min(b.y)),
                    eighth_pixels(a.x).saturating_add(radius),
                    eighth_pixels(a.y.max(b.y)),
                );
            }
        }
    }
    events.sort_unstable();
    y.sort_unstable();
    y.dedup();
    let end = y.len().saturating_sub(1);
    let intervals = vec![Interval::default(); 4 * y.len()];
    let mut sweep = Sweep { y, intervals };
    let mut previous = 0;
    let mut area = 0u128;
    for (x, bottom, top, add) in events {
        area += u128::from(x - previous) * u128::from(sweep.intervals[1].length);
        previous = x;
        let range = (
            sweep.y.binary_search(&bottom).unwrap(),
            sweep.y.binary_search(&top).unwrap(),
        );
        sweep.update(1, (0, end), range, add);
    }
    let viewport = width as f64 * height as f64;
    let occupied = area as f64 / 64.0;
    let whitespace = viewport - occupied;
    json!({
        "viewport_area": viewport, "occupied_area": occupied, "whitespace_area": whitespace,
        "aspect_ratio": (height != 0).then(|| width as f64 / height as f64),
        "occupancy_ratio": (viewport > 0.0).then(|| occupied / viewport),
        "whitespace_ratio": (viewport > 0.0).then(|| whitespace / viewport),
    })
}
