use super::{Bounds, Entry, Layout, Marker};
use crate::layout::Point;
use std::cmp::Ordering;

pub(super) fn place(layout: &Layout<'_>, entries: &[Entry<'_>]) -> Vec<Marker> {
    let mut markers = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let number = i + 1;
        let width = number.to_string().len() * 8 + 16;
        let height = 18;
        for &edge in &entry.edges {
            let Some(segment) = layout.paths[edge].windows(2).rev().find(|s| s[0] != s[1]) else {
                continue;
            };
            let (before, target) = (segment[0], segment[1]);
            let offset = if before.x == target.x {
                height / 2 + 4
            } else {
                width / 2 + 4
            };
            let center = Point {
                x: match before.x.cmp(&target.x) {
                    Ordering::Equal => target.x,
                    Ordering::Less => target.x.saturating_sub(offset),
                    Ordering::Greater => target.x + offset,
                },
                y: match before.y.cmp(&target.y) {
                    Ordering::Equal => target.y,
                    Ordering::Less => target.y.saturating_sub(offset),
                    Ordering::Greater => target.y + offset,
                },
            };
            markers.push(Marker {
                number,
                bounds: Bounds {
                    origin: Point {
                        x: center.x.saturating_sub(width / 2),
                        y: center.y.saturating_sub(height / 2),
                    },
                    width,
                    height,
                },
            });
        }
    }
    markers
}
