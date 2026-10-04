use super::super::{Bounds, Layout, NODE_WIDTH, Point};
use crate::model::{Edge, EdgeKind, Graph};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

fn ancestor(parents: &[Option<usize>], parent: usize, mut node: usize) -> bool {
    loop {
        if parent == node {
            return true;
        }
        match parents[node] {
            Some(next) => node = next,
            None => return false,
        }
    }
}

pub(super) fn crosses(a: Point, b: Point, bounds: Bounds) -> bool {
    let Point { x, y } = bounds.origin;
    match a.x == b.x {
        true => {
            a.x > x
                && a.x < x + bounds.width
                && a.y.max(b.y) > y
                && a.y.min(b.y) < y + bounds.height
        }
        false => {
            a.y > y
                && a.y < y + bounds.height
                && a.x.max(b.x) > x
                && a.x.min(b.x) < x + bounds.width
        }
    }
}

fn represented_by_nesting(edge: &Edge, parents: &[Option<usize>]) -> bool {
    matches!(edge.kind, EdgeKind::Dependency | EdgeKind::Containment)
        && edge.from != edge.to
        && (ancestor(parents, edge.from, edge.to) || ancestor(parents, edge.to, edge.from))
}

pub(super) fn incidents(graph: &Graph, parents: &[Option<usize>]) -> Vec<Vec<(usize, bool)>> {
    let mut incident = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        if !represented_by_nesting(edge, parents) {
            incident[edge.from].push((index, true));
            incident[edge.to].push((index, false));
        }
    }
    incident
}

pub(super) fn route(graph: &Graph, layout: &Layout<'_>, keys: &[usize]) -> Vec<Vec<Point>> {
    let mut right_edges: Vec<_> = layout
        .positions
        .iter()
        .map(|point| point.x + NODE_WIDTH)
        .collect();
    for &(node, bounds) in &layout.containers {
        right_edges[node] = bounds.origin.x + bounds.width;
    }
    let mut incident = incidents(graph, &layout.parents);
    for edges in &mut incident {
        edges.sort_by_key(|&(index, source)| {
            let edge = &graph.edges[index];
            let peer = match source {
                true => edge.to,
                false => edge.from,
            };
            (
                keys[peer],
                source,
                edge.kind,
                edge.change.as_ref().map(|change| change.action),
            )
        });
    }
    let mut source_ports = vec![0; graph.edges.len()];
    let mut target_ports = vec![0; graph.edges.len()];
    for (node, edges) in incident.iter().enumerate() {
        for (slot, &(edge, source)) in edges.iter().enumerate() {
            let port = 20 + (slot + 1) * (layout.header_heights[node] - 40) / (edges.len() + 1);
            match source {
                true => source_ports[edge] = port,
                false => target_ports[edge] = port,
            }
        }
    }
    graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            if represented_by_nesting(edge, &layout.parents) {
                return Vec::new();
            }
            let start = Point {
                x: right_edges[edge.from],
                y: layout.positions[edge.from].y + source_ports[index],
            };
            let end = Point {
                x: right_edges[edge.to],
                y: layout.positions[edge.to].y + target_ports[index],
            };
            let mut obstacles: Vec<_> = layout
                .positions
                .iter()
                .enumerate()
                .map(|(node, &origin)| Bounds {
                    origin,
                    width: NODE_WIDTH,
                    height: layout.header_heights[node],
                })
                .collect();
            obstacles.extend(
                layout
                    .containers
                    .iter()
                    .filter(|(node, _)| {
                        !ancestor(&layout.parents, *node, edge.from)
                            && !ancestor(&layout.parents, *node, edge.to)
                    })
                    .map(|(_, bounds)| *bounds),
            );
            let mut path = vec![start];
            path.extend(find_path(
                Point {
                    x: start.x + 16,
                    ..start
                },
                Point {
                    x: end.x + 16,
                    ..end
                },
                &obstacles,
            ));
            path.push(end);
            path
        })
        .collect()
}

fn find_path(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for bounds in obstacles {
        xs.extend([bounds.origin.x - 16, bounds.origin.x + bounds.width + 16]);
        ys.extend([bounds.origin.y - 16, bounds.origin.y + bounds.height + 16]);
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let index = |point: Point| {
        ys.binary_search(&point.y).unwrap() * xs.len() + xs.binary_search(&point.x).unwrap()
    };
    let point = |index: usize| Point {
        x: xs[index % xs.len()],
        y: ys[index / xs.len()],
    };
    let (first, last) = (index(start), index(end));
    let mut distances = vec![usize::MAX; xs.len() * ys.len()];
    let mut previous = vec![None; distances.len()];
    let mut queue = BinaryHeap::from([Reverse((0, first))]);
    distances[first] = 0;
    while let Some(Reverse((distance, current))) = queue.pop() {
        if current == last {
            break;
        }
        if distance != distances[current] {
            continue;
        }
        let (x, y) = (current % xs.len(), current / xs.len());
        let neighbors = [
            x.checked_sub(1).map(|x| y * xs.len() + x),
            (x + 1 < xs.len()).then_some(current + 1),
            y.checked_sub(1).map(|y| y * xs.len() + x),
            (y + 1 < ys.len()).then_some(current + xs.len()),
        ];
        for next in neighbors.into_iter().flatten() {
            let (a, b) = (point(current), point(next));
            if obstacles.iter().any(|&bounds| crosses(a, b, bounds)) {
                continue;
            }
            let candidate = distance + a.x.abs_diff(b.x) + a.y.abs_diff(b.y);
            if candidate < distances[next] {
                distances[next] = candidate;
                previous[next] = Some(current);
                queue.push(Reverse((candidate, next)));
            }
        }
    }
    let mut path = vec![end];
    let mut current = last;
    while current != first {
        current =
            previous[current].expect("nested layout leaves routing corridors around every card");
        path.push(point(current));
    }
    path.reverse();
    let mut compact: Vec<Point> = Vec::new();
    for point in path {
        if compact.len() >= 2 {
            let (a, b) = (compact[compact.len() - 2], compact[compact.len() - 1]);
            if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) {
                compact.pop();
            }
        }
        compact.push(point);
    }
    compact
}
