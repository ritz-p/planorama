use super::super::{Bounds, Layout, NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::{Edge, EdgeKind, Graph};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

fn ancestor(layout: &Layout<'_>, parent: usize, mut node: usize) -> bool {
    loop {
        if parent == node {
            return true;
        }
        match layout.parents[node] {
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

fn represented_by_nesting(edge: &Edge, layout: &Layout<'_>) -> bool {
    matches!(edge.kind, EdgeKind::Dependency | EdgeKind::Containment)
        && edge.from != edge.to
        && (ancestor(layout, edge.from, edge.to) || ancestor(layout, edge.to, edge.from))
}

pub(super) fn route(graph: &Graph, layout: &Layout<'_>) -> Vec<Vec<Point>> {
    let mut outgoing = vec![Vec::new(); graph.nodes.len()];
    let mut incoming = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        if !represented_by_nesting(edge, layout) {
            outgoing[edge.from].push(index);
            incoming[edge.to].push(index);
        }
    }
    let mut source_ports = vec![0; graph.edges.len()];
    let mut target_ports = vec![0; graph.edges.len()];
    for (groups, ports) in [
        (&outgoing, &mut source_ports),
        (&incoming, &mut target_ports),
    ] {
        for edges in groups {
            for (slot, &edge) in edges.iter().enumerate() {
                ports[edge] = 20 + (slot + 1) * (NODE_HEIGHT - 40) / (edges.len() + 1);
            }
        }
    }
    graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            if represented_by_nesting(edge, layout) {
                return Vec::new();
            }
            let start = Point {
                x: layout.positions[edge.from].x + NODE_WIDTH,
                y: layout.positions[edge.from].y + source_ports[index],
            };
            let end = Point {
                x: layout.positions[edge.to].x + NODE_WIDTH,
                y: layout.positions[edge.to].y
                    + match edge.from == edge.to {
                        true => NODE_HEIGHT - 12,
                        false => target_ports[index],
                    },
            };
            let mut obstacles: Vec<_> = layout
                .positions
                .iter()
                .map(|&origin| Bounds {
                    origin,
                    width: NODE_WIDTH,
                    height: NODE_HEIGHT,
                })
                .collect();
            obstacles.extend(
                layout
                    .containers
                    .iter()
                    .filter(|(node, _)| {
                        !ancestor(layout, *node, edge.from) && !ancestor(layout, *node, edge.to)
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
