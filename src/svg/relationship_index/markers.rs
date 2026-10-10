use super::{Bounds, Entry, Graph, Layout, Marker};
use crate::layout::Point;
use crate::layout::relationship_markers::{GAP, HEIGHT, width};
use std::cmp::Ordering;

pub(super) fn place(graph: &Graph, layout: &Layout<'_>, entries: &[Entry<'_>]) -> Vec<Marker> {
    let mut markers = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let number = i + 1;
        let width = width(number);
        let height = HEIGHT;
        for &edge in &entry.edges {
            let path = &layout.paths[edge];
            let target_segment = path
                .windows(2)
                .rev()
                .find(|s| s[0] != s[1])
                .map(|s| (s[0], s[1]));
            let source_segment = (graph.edges[edge].directionality()
                == crate::model::Directionality::Undirected)
                .then(|| path.windows(2).find(|s| s[0] != s[1]).map(|s| (s[1], s[0])))
                .flatten();
            for segment in [target_segment, source_segment] {
                let Some((before, target)) = segment else {
                    continue;
                };
                let offset = if before.x == target.x {
                    height / 2 + GAP
                } else {
                    width / 2 + GAP
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
                    stroke: super::super::relationships::stroke(&graph.edges[edge]),
                    style: super::super::relationships::style(graph.edges[edge].kind),
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
    }
    markers
}
