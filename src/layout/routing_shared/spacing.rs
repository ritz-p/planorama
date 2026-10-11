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

fn barriers(graph: &Graph, layout: &Layout<'_>, edge: usize) -> Vec<Bounds> {
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

fn shift(
    graph: &Graph,
    layout: &mut Layout<'_>,
    segment: Segment,
    coordinate: usize,
    baseline: usize,
) -> bool {
    let path = &layout.paths[segment.edge];
    let i = segment.index;
    if i == 0 || i + 2 >= path.len() {
        return false;
    }
    if layout
        .junctions
        .iter()
        .any(|p| *p == path[i] || *p == path[i + 1])
    {
        return false;
    }
    for index in i - 1..=i + 1 {
        let p = &path[index..=index + 1];
        let horizontal = p[0].y == p[1].y;
        let (coordinate, low, high) = if horizontal {
            (p[0].y, p[0].x.min(p[1].x), p[0].x.max(p[1].x))
        } else {
            (p[0].x, p[0].y.min(p[1].y), p[0].y.max(p[1].y))
        };
        let a = Segment {
            edge: segment.edge,
            index,
            horizontal,
            coordinate,
            low,
            high,
        };
        for (other, points) in layout.paths.iter().enumerate() {
            if other == segment.edge {
                continue;
            }
            for q in points.windows(2) {
                let (c, lo, hi) = if horizontal && q[0].y == q[1].y {
                    (q[0].y, q[0].x.min(q[1].x), q[0].x.max(q[1].x))
                } else if !horizontal && q[0].x == q[1].x {
                    (q[0].x, q[0].y.min(q[1].y), q[0].y.max(q[1].y))
                } else {
                    continue;
                };
                if c == coordinate
                    && low.max(lo) < high.min(hi)
                    && shared(
                        graph,
                        &layout.paths,
                        a,
                        Segment { edge: other, ..a },
                        low.max(lo),
                        high.min(hi),
                    )
                {
                    return false;
                }
            }
        }
    }
    let mut candidate = path.clone();
    for p in &mut candidate[i..=i + 1] {
        if segment.horizontal {
            p.y = coordinate;
        } else {
            p.x = coordinate;
        }
    }
    candidate = simplify(candidate, Simplification::PreserveReversals);
    if candidate.len() > path.len()
        || candidate.windows(3).any(|p| {
            (p[0].x == p[1].x && p[1].x == p[2].x) || (p[0].y == p[1].y && p[1].y == p[2].y)
        })
        || length(&candidate) > baseline + 64
        || candidate
            .iter()
            .any(|p| p.x >= layout.width || p.y >= layout.height)
    {
        return false;
    }
    let numbered = crate::layout::relationship_markers::ends(graph);
    let clearance = crate::layout::relationship_markers::clearance(graph);
    if !ports::valid_with_clearances(
        &candidate,
        terminal(path, 0),
        terminal(path, 1),
        &barriers(graph, layout, segment.edge),
        numbered[segment.edge].map(|n| if n { clearance } else { 16 }),
    ) {
        return false;
    }
    let before = measure(graph, &layout.paths);
    let overlap = |p: &[Point]| {
        layout
            .paths
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != segment.edge)
            .map(|(_, other)| super::refinement::overlap(p, other))
            .sum::<usize>()
    };
    if overlap(&candidate) > overlap(path) {
        return false;
    }
    let old = std::mem::replace(&mut layout.paths[segment.edge], candidate);
    let after = measure(graph, &layout.paths);
    if after.deficit < before.deficit && after.close_length <= before.close_length {
        true
    } else {
        layout.paths[segment.edge] = old;
        false
    }
}

fn expand(
    graph: &Graph,
    layout: &mut Layout<'_>,
    horizontal: bool,
    cut: usize,
    amount: usize,
    baseline: &[usize],
) -> bool {
    let axis = |p: Point| if horizontal { p.y } else { p.x };
    for (node, b) in layout.bounds.iter().enumerate() {
        let (low, high) = if horizontal {
            (b.origin.y, b.origin.y + b.height)
        } else {
            (b.origin.x, b.right())
        };
        if low <= cut
            && cut <= high
            && (!layout.containers.contains(&node)
                || (horizontal && low < cut && cut <= b.origin.y + layout.header_heights[node]))
        {
            return false;
        }
    }
    if horizontal
        && layout
            .scopes
            .iter()
            .any(|s| s.bounds.origin.y < cut && cut < s.bounds.origin.y + 36)
    {
        return false;
    }
    let numbered = crate::layout::relationship_markers::ends(graph);
    let clearance = crate::layout::relationship_markers::clearance(graph);
    for (i, path) in layout.paths.iter().enumerate() {
        if path.len() < 2 {
            continue;
        }
        for b in ports::terminal_corridors(
            terminal(path, 0),
            terminal(path, 1),
            numbered[i].map(|n| if n { clearance } else { 16 }),
        ) {
            let (lo, hi) = if horizontal {
                (b.origin.y, b.origin.y + b.height)
            } else {
                (b.origin.x, b.right())
            };
            if lo < cut && cut < hi {
                return false;
            }
        }
    }
    let move_point = |p: &mut Point| {
        if axis(*p) >= cut {
            if horizontal {
                p.y += amount;
            } else {
                p.x += amount;
            }
        }
    };
    let move_bounds = |b: &mut Bounds| {
        let (low, high) = if horizontal {
            (b.origin.y, b.origin.y + b.height)
        } else {
            (b.origin.x, b.right())
        };
        if low >= cut {
            move_point(&mut b.origin);
        } else if high >= cut {
            if horizontal {
                b.height += amount;
            } else {
                b.width += amount;
            }
        }
    };
    let mut paths = layout.paths.clone();
    for p in paths.iter_mut().flatten() {
        move_point(p);
    }
    if paths
        .iter()
        .zip(baseline)
        .any(|(p, &base)| length(p) > base + 128)
    {
        return false;
    }
    let before = measure(graph, &layout.paths);
    let after = measure(graph, &paths);
    if after.deficit >= before.deficit || after.close_length > before.close_length {
        return false;
    }
    for b in &mut layout.bounds {
        move_bounds(b);
    }
    for s in &mut layout.scopes {
        move_bounds(&mut s.bounds);
    }
    layout.paths = paths;
    for p in &mut layout.junctions {
        move_point(p);
    }
    for band in &mut layout.bands {
        if horizontal {
            if band.top >= cut {
                band.top += amount;
            } else if band.top + band.height > cut {
                band.height += amount;
            }
        }
    }
    if horizontal {
        layout.height += amount;
    } else {
        layout.width += amount;
    }
    layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    true
}

pub(in crate::layout) fn refine(graph: &Graph, layout: &mut Layout<'_>) {
    let baseline: Vec<_> = layout.paths.iter().map(|p| length(p)).collect();
    let mut growth = [0; 2];
    for _ in 0..16 {
        let mut congested = pairs(graph, &layout.paths, PREFERRED - 1);
        let identity = |s: Segment| {
            let e = &graph.edges[s.edge];
            (
                graph.nodes[e.from].entity.id.as_str(),
                graph.nodes[e.to].entity.id.as_str(),
                e.kind,
                e.change.as_ref().map(|c| (c.address.as_str(), c.action)),
                s.index,
            )
        };
        for (a, b, _) in &mut congested {
            if a.coordinate == b.coordinate && identity(*b) < identity(*a) {
                std::mem::swap(a, b);
            }
        }
        congested.sort_by_key(|(a, b, _)| {
            (
                a.horizontal,
                a.coordinate,
                b.coordinate,
                identity(*a),
                identity(*b),
            )
        });
        let mut changed = false;
        for (a, b, _) in congested.into_iter().take(32) {
            for (segment, other) in [(a, b), (b, a)] {
                for coordinate in [
                    other.coordinate.saturating_sub(PREFERRED),
                    other.coordinate.saturating_add(PREFERRED),
                ] {
                    if shift(graph, layout, segment, coordinate, baseline[segment.edge]) {
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }
            if changed {
                break;
            }
            let distance = b.coordinate - a.coordinate;
            if distance > 0
                && growth[usize::from(a.horizontal)] + PREFERRED - distance <= 64
                && expand(
                    graph,
                    layout,
                    a.horizontal,
                    a.coordinate + distance.div_ceil(2),
                    PREFERRED - distance,
                    &baseline,
                )
            {
                growth[usize::from(a.horizontal)] += PREFERRED - distance;
                changed = true;
                break;
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
