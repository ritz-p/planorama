use super::{Simplification, ports, refinement::terminal, simplify};
use crate::layout::{Bounds, Layout, Point};
use crate::model::Graph;
use std::collections::BTreeMap;

pub(in crate::layout) const PREFERRED: usize = 16;
const RUN: usize = 64;
const CONVERGENCE: usize = 32;

#[derive(Clone, Copy, Debug)]
struct Segment {
    edge: usize,
    index: usize,
    horizontal: bool,
    coordinate: usize,
    low: usize,
    high: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::layout) struct Metrics {
    pub minimum: Option<usize>,
    pub close_length: usize,
    pub deficit: usize,
}

fn shared_prefix(
    a: &[Point],
    b: &[Point],
    horizontal: bool,
    coordinate: usize,
    low: usize,
    high: usize,
) -> bool {
    if a.first() != b.first() || a.len() < 2 || b.len() < 2 {
        return false;
    }
    let mut current = a[0];
    let (mut i, mut j) = (1, 1);
    while i < a.len() && j < b.len() {
        let (x, y) = (a[i], b[j]);
        if (x.x.cmp(&current.x), x.y.cmp(&current.y)) != (y.x.cmp(&current.x), y.y.cmp(&current.y))
        {
            break;
        }
        let distance = |p: Point| p.x.abs_diff(current.x) + p.y.abs_diff(current.y);
        let next = if distance(x) <= distance(y) { x } else { y };
        let (c, lo, hi) = if horizontal && current.y == next.y {
            (current.y, current.x.min(next.x), current.x.max(next.x))
        } else if !horizontal && current.x == next.x {
            (current.x, current.y.min(next.y), current.y.max(next.y))
        } else {
            (usize::MAX, 0, 0)
        };
        if c == coordinate && lo <= low && high <= hi {
            return true;
        }
        if next == x {
            i += 1;
        }
        if next == y {
            j += 1;
        }
        current = next;
    }
    false
}

fn shared(
    graph: &Graph,
    paths: &[Vec<Point>],
    a: Segment,
    b: Segment,
    low: usize,
    high: usize,
) -> bool {
    if a.coordinate != b.coordinate {
        return false;
    }
    let (ea, eb) = (&graph.edges[a.edge], &graph.edges[b.edge]);
    if ea.from == eb.from
        && shared_prefix(
            &paths[a.edge],
            &paths[b.edge],
            a.horizontal,
            a.coordinate,
            low,
            high,
        )
    {
        return true;
    }
    ea.to == eb.to
        && shared_prefix(
            &paths[a.edge].iter().copied().rev().collect::<Vec<_>>(),
            &paths[b.edge].iter().copied().rev().collect::<Vec<_>>(),
            a.horizontal,
            a.coordinate,
            low,
            high,
        )
}

fn segments(paths: &[Vec<Point>]) -> BTreeMap<(bool, usize), Vec<Segment>> {
    let mut result = BTreeMap::<_, Vec<_>>::new();
    for (edge, path) in paths.iter().enumerate() {
        for (index, p) in path.windows(2).enumerate() {
            let horizontal = p[0].y == p[1].y;
            let (from, to, coordinate) = if horizontal {
                (p[0].x, p[1].x, p[0].y)
            } else {
                (p[0].y, p[1].y, p[0].x)
            };
            let (mut low, mut high) = (from.min(to), from.max(to));
            for (endpoint, trim) in [(from, index == 0), (to, index + 2 == path.len())] {
                if trim {
                    if endpoint == from.min(to) {
                        low = low.saturating_add(CONVERGENCE);
                    } else {
                        high = high.saturating_sub(CONVERGENCE);
                    }
                }
            }
            if high.saturating_sub(low) >= RUN {
                result
                    .entry((horizontal, coordinate))
                    .or_default()
                    .push(Segment {
                        edge,
                        index,
                        horizontal,
                        coordinate,
                        low,
                        high,
                    });
            }
        }
    }
    result
}

fn pairs(graph: &Graph, paths: &[Vec<Point>], distance: usize) -> Vec<(Segment, Segment, usize)> {
    let index = segments(paths);
    let mut result = Vec::new();
    for (&(horizontal, coordinate), group) in &index {
        for a in group {
            for (_, others) in index
                .range((horizontal, coordinate)..=(horizontal, coordinate.saturating_add(distance)))
            {
                for b in others {
                    if a.edge == b.edge
                        || (coordinate == b.coordinate && (a.edge, a.index) >= (b.edge, b.index))
                    {
                        continue;
                    }
                    let (low, high) = (a.low.max(b.low), a.high.min(b.high));
                    if high.saturating_sub(low) < RUN || shared(graph, paths, *a, *b, low, high) {
                        continue;
                    }
                    result.push((*a, *b, high - low));
                }
            }
        }
    }
    result
}

pub(in crate::layout) fn measure(graph: &Graph, paths: &[Vec<Point>]) -> Metrics {
    let mut result = Metrics::default();
    for (a, b, length) in pairs(graph, paths, PREFERRED) {
        let distance = a.coordinate.abs_diff(b.coordinate);
        result.minimum = Some(result.minimum.map_or(distance, |n| n.min(distance)));
        if distance < PREFERRED {
            result.close_length += length;
            result.deficit += length * (PREFERRED - distance);
        }
    }
    result
}

fn length(path: &[Point]) -> usize {
    path.windows(2)
        .map(|p| p[0].x.abs_diff(p[1].x) + p[0].y.abs_diff(p[1].y))
        .sum()
}

pub(in crate::layout) fn barriers(graph: &Graph, layout: &Layout<'_>, edge: usize) -> Vec<Bounds> {
    let mut result = crate::layout::containers::routing::obstacles(layout, &graph.edges[edge]);
    result.extend(
        layout
            .bounds
            .iter()
            .enumerate()
            .filter(|(n, _)| !layout.containers.contains(n))
            .map(|(_, b)| *b),
    );
    result.extend(
        layout
            .occluders(&graph.edges[edge])
            .into_iter()
            .map(|n| layout.bounds[n]),
    );
    let numbered = crate::layout::relationship_markers::ends(graph);
    let clearance = crate::layout::relationship_markers::clearance(graph);
    for (i, path) in layout.paths.iter().enumerate() {
        if i == edge || path.len() < 2 {
            continue;
        }
        let p = [terminal(path, 0), terminal(path, 1)];
        result.extend(ports::terminal_corridors(
            p[0],
            p[1],
            numbered[i].map(|n| if n { clearance } else { 16 }),
        ));
        result.extend(p.iter().map(|p| Bounds {
            origin: Point {
                x: p.point.x.saturating_sub(1),
                y: p.point.y.saturating_sub(1),
            },
            width: 2,
            height: 2,
        }));
    }
    result
}

mod expansion;
mod plan;
mod quality;
mod reroute;
pub(in crate::layout) use quality::Shape;

pub(in crate::layout) fn refine(graph: &Graph, layout: &mut Layout<'_>) {
    let original = (layout.width, layout.height);
    let baseline: Vec<_> = layout.paths.iter().map(|p| Shape::measure(p)).collect();
    let mut attempts = 0;
    let mut attempted = std::collections::BTreeSet::new();
    for _ in 0..2 {
        let corridors = plan::corridors(graph, layout);
        let mut changed = false;
        for corridor in corridors {
            if !attempted.insert((
                corridor.horizontal,
                corridor
                    .lanes
                    .iter()
                    .map(|l| (l.edge, l.original))
                    .collect::<Vec<_>>(),
            )) {
                continue;
            }
            if attempts >= 6 {
                return;
            }
            attempts += 1;
            let before = expansion::Snapshot::capture(layout);
            let mut planned = corridor.clone();
            let mut expanded = false;
            if let Some((cut, amount)) = planned.demand(layout) {
                let current = if planned.horizontal {
                    layout.height - original.1
                } else {
                    layout.width - original.0
                };
                if current + amount <= 64
                    && expansion::allocate(layout, planned.horizontal, cut, amount)
                {
                    planned.translate(cut, amount);
                    expanded = true;
                }
            }
            planned.allocate(layout);
            if reroute::apply(graph, layout, &planned, &before, &baseline, expanded)
                && quality::improves(graph, &before.paths, &layout.paths, expanded)
            {
                changed = true;
                continue;
            }
            before.restore(layout);
            if expanded {
                let fallback = expansion::Snapshot::capture(layout);
                let mut planned = corridor;
                planned.allocate(layout);
                if reroute::apply(graph, layout, &planned, &fallback, &baseline, false)
                    && quality::improves(graph, &fallback.paths, &layout.paths, false)
                {
                    changed = true;
                } else {
                    fallback.restore(layout);
                }
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/spacing.rs"]
mod tests;
