use super::super::{Bounds, Layout, Point};
use crate::layout::routing::scoring::Scorer;
use crate::model::{Edge, EdgeKind, Graph};
mod search;
use search::find_path;

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
    route_with_quality(graph, layout, keys, true)
}

pub(super) fn route_with_quality(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
) -> Vec<Vec<Point>> {
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
                edge.change.as_ref().map(|change| change.local_address()),
            )
        });
    }
    let mut source_ports = vec![0; graph.edges.len()];
    let mut target_ports = vec![0; graph.edges.len()];
    for (node, edges) in incident.iter().enumerate() {
        for (slot, &(edge, source)) in edges.iter().enumerate() {
            let port = 20 + (slot + 1) * (layout.bounds[node].height - 40) / (edges.len() + 1);
            match source {
                true => source_ports[edge] = port,
                false => target_ports[edge] = port,
            }
        }
    }
    let mut order: Vec<_> = (0..graph.edges.len()).collect();
    order.sort_by_key(|&index| {
        let edge = &graph.edges[index];
        (
            keys[edge.from],
            keys[edge.to],
            edge.kind,
            edge.change.as_ref().map(|c| c.action),
            edge.change.as_ref().map(|c| c.local_address()),
        )
    });
    let mut scorer = Scorer::default();
    let mut paths = vec![Vec::new(); graph.edges.len()];
    for index in order {
        let edge = &graph.edges[index];
        if represented_by_nesting(edge, &layout.parents) {
            continue;
        }
        let start = Point {
            x: layout.bounds[edge.from].right(),
            y: layout.bounds[edge.from].origin.y + source_ports[index],
        };
        let end = Point {
            x: layout.bounds[edge.to].right(),
            y: layout.bounds[edge.to].origin.y + target_ports[index],
        };
        let obstacles = obstacles(layout, edge);
        let path = path_between(start, end, &obstacles, quality.then_some(&scorer));
        scorer.insert(path.clone());
        paths[index] = path;
    }
    paths
}

pub(super) fn obstacles(layout: &Layout<'_>, edge: &Edge) -> Vec<Bounds> {
    layout
        .bounds
        .iter()
        .enumerate()
        .map(|(node, &bounds)| {
            if ancestor(&layout.parents, node, edge.from)
                || ancestor(&layout.parents, node, edge.to)
            {
                bounds.header(layout.header_heights[node])
            } else {
                bounds
            }
        })
        .collect()
}

pub(super) fn path_between(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    let first = Point {
        x: start.x + 16,
        ..start
    };
    let last = Point {
        x: end.x + 16,
        ..end
    };
    let finish = |middle: Vec<Point>| {
        let mut path = vec![start];
        path.extend(middle);
        path.push(end);
        path
    };
    let shortest = finish(find_path(first, last, obstacles, None));
    match scorer {
        Some(scorer) => {
            let candidate = finish(find_path(first, last, obstacles, Some(scorer)));
            if scorer.readability_cost(&candidate) < scorer.readability_cost(&shortest) {
                candidate
            } else {
                shortest
            }
        }
        None => shortest,
    }
}
