use super::{Bounds, Entry, Graph, Layout, Marker};
use crate::layout::Point;
use crate::model::ResourceRole;
use std::cmp::Ordering;

pub(super) fn place(graph: &Graph, layout: &Layout<'_>, entries: &[Entry<'_>]) -> Vec<Marker> {
    let mut occupied = Vec::new();
    for panel in &layout.scopes {
        occupied.push(panel.bounds.header(36));
        occupied.extend(borders(panel.bounds));
    }
    for (index, bounds) in layout.bounds.iter().copied().enumerate() {
        occupied.push(bounds.header(layout.header_heights[index]));
        if graph.nodes[index].role == ResourceRole::Container {
            occupied.extend(borders(bounds));
        }
    }
    for point in layout.junctions.iter().chain(
        layout
            .paths
            .iter()
            .flat_map(|p| p.first().into_iter().chain(p.last())),
    ) {
        occupied.push(Bounds {
            origin: Point {
                x: point.x.saturating_sub(12),
                y: point.y.saturating_sub(12),
            },
            width: 24,
            height: 24,
        });
    }
    let mut markers = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let number = i + 1;
        let width = number.to_string().len() * 8 + 16;
        for &edge in &entry.edges {
            let path = &layout.paths[edge];
            let mut best = None;
            let mut remaining = 0;
            for segment in path.windows(2).rev() {
                let (a, b) = (segment[0], segment[1]);
                let length = distance(a, b);
                let half = if a.x == b.x { 11 } else { width / 2 };
                for step in half + 6..length.saturating_sub(half + 5) {
                    let point = Point {
                        x: match a.x.cmp(&b.x) {
                            Ordering::Equal => b.x,
                            Ordering::Less => b.x - step,
                            Ordering::Greater => b.x + step,
                        },
                        y: match a.y.cmp(&b.y) {
                            Ordering::Equal => b.y,
                            Ordering::Less => b.y - step,
                            Ordering::Greater => b.y + step,
                        },
                    };
                    let bounds = Bounds {
                        origin: Point {
                            x: point.x.saturating_sub(width / 2),
                            y: point.y.saturating_sub(11),
                        },
                        width,
                        height: 22,
                    };
                    let obstruction: usize =
                        occupied.iter().map(|&b| overlap_area(bounds, b)).sum();
                    let crossings = layout
                        .paths
                        .iter()
                        .enumerate()
                        .filter(|(index, p)| {
                            *index != edge && p.windows(2).any(|s| crosses(s[0], s[1], bounds))
                        })
                        .count();
                    let score = (obstruction, crossings, remaining + step);
                    if best.as_ref().is_none_or(|(old, _)| score < *old) {
                        best = Some((score, bounds));
                    }
                    if obstruction == 0 && crossings == 0 {
                        break;
                    }
                }
                if best
                    .as_ref()
                    .is_some_and(|(score, _)| score.0 == 0 && score.1 == 0)
                {
                    break;
                }
                remaining += length;
            }
            let bounds = best.map(|(_, bounds)| bounds).unwrap_or_else(|| {
                let a = path[path.len() - 2];
                let b = path[path.len() - 1];
                let point = Point {
                    x: (a.x + b.x) / 2,
                    y: (a.y + b.y) / 2,
                };
                Bounds {
                    origin: Point {
                        x: point.x.saturating_sub(width / 2),
                        y: point.y.saturating_sub(11),
                    },
                    width,
                    height: 22,
                }
            });
            let marker = Marker { number, bounds };
            occupied.push(marker.bounds);
            markers.push(marker);
        }
    }
    markers
}

fn borders(bounds: Bounds) -> [Bounds; 3] {
    [
        Bounds { width: 4, ..bounds },
        Bounds {
            origin: Point {
                x: bounds.right().saturating_sub(4),
                y: bounds.origin.y,
            },
            width: 4,
            ..bounds
        },
        Bounds {
            origin: Point {
                x: bounds.origin.x,
                y: bounds.origin.y + bounds.height.saturating_sub(4),
            },
            height: 4,
            ..bounds
        },
    ]
}

fn distance(a: Point, b: Point) -> usize {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}

fn overlap_area(a: Bounds, b: Bounds) -> usize {
    a.right()
        .min(b.right())
        .saturating_sub(a.origin.x.max(b.origin.x))
        * (a.origin.y + a.height)
            .min(b.origin.y + b.height)
            .saturating_sub(a.origin.y.max(b.origin.y))
}
fn crosses(a: Point, b: Point, bounds: Bounds) -> bool {
    if a.x == b.x {
        a.x >= bounds.origin.x
            && a.x <= bounds.right()
            && a.y.max(b.y) >= bounds.origin.y
            && a.y.min(b.y) <= bounds.origin.y + bounds.height
    } else {
        a.y >= bounds.origin.y
            && a.y <= bounds.origin.y + bounds.height
            && a.x.max(b.x) >= bounds.origin.x
            && a.x.min(b.x) <= bounds.right()
    }
}
