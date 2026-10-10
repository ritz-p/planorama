use super::{Bounds, Entry, Graph, Layout, Marker, Point};
use crate::model::ResourceRole;

pub(super) fn place(graph: &Graph, layout: &Layout<'_>, entries: &[Entry<'_>]) -> Vec<Marker> {
    let mut occupied = Vec::new();
    for (index, bounds) in layout.bounds.iter().copied().enumerate() {
        occupied.push(bounds.header(layout.header_heights[index]));
        if graph.nodes[index].role == ResourceRole::Container {
            occupied.extend([
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
            ]);
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
        let width = number.to_string().len() * 8 + 24;
        for &edge in &entry.edges {
            let path = &layout.paths[edge];
            let mut segments: Vec<_> = path.windows(2).collect();
            segments.sort_by_key(|s| std::cmp::Reverse(distance(s[0], s[1])));
            let mut placed = None;
            'search: for segment in &segments {
                let (a, b) = (segment[0], segment[1]);
                let length = distance(a, b);
                for step in
                    std::iter::once(length / 2).chain((12..length.saturating_sub(12)).step_by(12))
                {
                    let point = if a.x == b.x {
                        Point {
                            x: a.x,
                            y: a.y.min(b.y) + step,
                        }
                    } else {
                        Point {
                            x: a.x.min(b.x) + step,
                            y: a.y,
                        }
                    };
                    let candidates = if a.x == b.x {
                        [
                            (point.x as isize + 6, point.y as isize - 11),
                            (point.x as isize - width as isize - 6, point.y as isize - 11),
                            (point.x as isize - width as isize / 2, point.y as isize - 11),
                        ]
                    } else {
                        [
                            (point.x as isize - width as isize / 2, point.y as isize - 28),
                            (point.x as isize - width as isize / 2, point.y as isize + 6),
                            (point.x as isize - width as isize / 2, point.y as isize - 11),
                        ]
                    };
                    for (x, y) in candidates {
                        if x < 20 || y < 150 {
                            continue;
                        }
                        let bounds = Bounds {
                            origin: Point {
                                x: x as usize,
                                y: y as usize,
                            },
                            width,
                            height: 22,
                        };
                        if bounds.right() + 20 > layout.width
                            || bounds.origin.y + 30 > layout.height
                            || occupied.iter().any(|&b| overlaps(bounds, b))
                            || layout.paths.iter().enumerate().any(|(index, p)| {
                                index != edge && p.windows(2).any(|s| crosses(s[0], s[1], bounds))
                            })
                        {
                            continue;
                        }
                        placed = Some(Marker {
                            number,
                            bounds,
                            leader: Vec::new(),
                        });
                        break 'search;
                    }
                }
            }
            let marker = placed.unwrap_or_else(|| {
                let segment = segments[0];
                let anchor = Point {
                    x: (segment[0].x + segment[1].x) / 2,
                    y: (segment[0].y + segment[1].y) / 2,
                };
                let mut column = 0;
                let bounds = 'slots: loop {
                    for y in std::iter::once(
                        anchor
                            .y
                            .clamp(160, layout.height.saturating_sub(30).max(160)),
                    )
                    .chain((160..layout.height.saturating_sub(30).max(190)).step_by(30))
                    {
                        let bounds = Bounds {
                            origin: Point {
                                x: layout.width + 20 + column * (width + 12),
                                y,
                            },
                            width,
                            height: 22,
                        };
                        if !occupied.iter().any(|&b| overlaps(bounds, b)) {
                            break 'slots bounds;
                        }
                    }
                    column += 1;
                };
                let obstacles: Vec<_> = layout
                    .bounds
                    .iter()
                    .enumerate()
                    .map(|(i, bounds)| bounds.header(layout.header_heights[i]))
                    .chain(markers.iter().map(|marker: &Marker| marker.bounds))
                    .filter(|b| !contains(*b, anchor))
                    .collect();
                let end = Point {
                    x: bounds.origin.x,
                    y: bounds.origin.y + 11,
                };
                let leader = crate::layout::route_to_margin(anchor, end, &obstacles);
                Marker {
                    number,
                    bounds,
                    leader,
                }
            });
            occupied.push(marker.bounds);
            markers.push(marker);
        }
    }
    markers
}

fn distance(a: Point, b: Point) -> usize {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}

fn contains(bounds: Bounds, point: Point) -> bool {
    point.x >= bounds.origin.x
        && point.x <= bounds.right()
        && point.y >= bounds.origin.y
        && point.y <= bounds.origin.y + bounds.height
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.origin.x < b.right() + 3
        && b.origin.x < a.right() + 3
        && a.origin.y < b.origin.y + b.height + 3
        && b.origin.y < a.origin.y + a.height + 3
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
