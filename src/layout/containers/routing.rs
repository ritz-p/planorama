use super::super::{Bounds, Layout, Point};
use crate::layout::routing::scoring::Scorer;
use crate::model::{Edge, EdgeKind, Graph};
mod bundles;
mod search;
use search::find_path;
pub(super) fn to_margin(start: Point, end: Point, obstacles: &[Bounds]) -> Vec<Point> {
    search::to_margin(start, end, obstacles)
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

fn represented_by_nesting(edge: &Edge, tree: &crate::layout::ContainmentTree) -> bool {
    matches!(edge.kind, EdgeKind::Dependency | EdgeKind::Containment)
        && edge.from != edge.to
        && (tree.is_ancestor(edge.from, edge.to) || tree.is_ancestor(edge.to, edge.from))
}

pub(super) fn incidents(
    graph: &Graph,
    tree: &crate::layout::ContainmentTree,
) -> Vec<Vec<(usize, bool)>> {
    let mut incident = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        if !represented_by_nesting(edge, tree) {
            incident[edge.from].push((index, true));
            incident[edge.to].push((index, false));
        }
    }
    incident
}

pub(super) struct Routed {
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

pub(super) fn route(graph: &Graph, layout: &Layout<'_>, keys: &[usize], bundle: bool) -> Routed {
    route_impl(graph, layout, keys, true, bundle)
}

#[cfg(test)]
pub(super) fn route_with_quality(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
) -> Vec<Vec<Point>> {
    route_impl(graph, layout, keys, quality, false).paths
}

fn route_impl(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
) -> Routed {
    // Structural keys intentionally tie for symmetric nodes. Final geometry
    // distinguishes those peers without depending on edge input order.
    let geometry = |node: usize| {
        let bounds = layout.bounds[node];
        (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
        )
    };
    let mut incident = incidents(graph, &layout.containment);
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
                geometry(peer),
                graph.nodes[peer].entity.id.as_str(),
                edge.change.as_ref().map(|change| change.address.as_str()),
            )
        });
    }
    let mut source_ports = vec![0; graph.edges.len()];
    let mut target_ports = vec![0; graph.edges.len()];
    for (node, edges) in incident.iter().enumerate() {
        for (slot, &(edge, source)) in edges.iter().enumerate() {
            let height = if bundle && source && graph.edges[edge].kind == EdgeKind::Connection {
                layout.header_heights[node]
            } else {
                layout.bounds[node].height
            };
            let port = 20 + (slot + 1) * (height - 40) / (edges.len() + 1);
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
            geometry(edge.from),
            geometry(edge.to),
            (
                graph.nodes[edge.from].entity.id.as_str(),
                graph.nodes[edge.to].entity.id.as_str(),
            ),
            edge.change.as_ref().map(|change| change.address.as_str()),
        )
    });
    let mut scorer = Scorer::default();
    let mut paths = vec![Vec::new(); graph.edges.len()];
    let groups = super::affinity::groups(graph, &layout.containment.parents);
    let mut attempted = std::collections::BTreeSet::new();
    let mut junctions = Vec::new();
    for index in order {
        let edge = &graph.edges[index];
        if represented_by_nesting(edge, &layout.containment) {
            continue;
        }
        if !paths[index].is_empty() {
            continue;
        }
        if bundle {
            if let Some(group) = groups.iter().find(|group| group.edges.contains(&index)) {
                if attempted.insert(group.target) {
                    if let Some(bundled) = bundles::try_bundle(
                        graph,
                        layout,
                        group,
                        &source_ports,
                        &target_ports,
                        &scorer,
                    ) {
                        for (index, path) in bundled.paths {
                            scorer.insert(path.clone());
                            paths[index] = path;
                        }
                        junctions.extend(bundled.junctions);
                        continue;
                    }
                }
            }
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
    junctions.sort_by_key(|point| (point.x, point.y));
    junctions.dedup();
    Routed { paths, junctions }
}

pub(super) fn obstacles(layout: &Layout<'_>, edge: &Edge) -> Vec<Bounds> {
    layout
        .bounds
        .iter()
        .enumerate()
        .map(|(node, &bounds)| {
            if layout.containment.is_ancestor(node, edge.from)
                || layout.containment.is_ancestor(node, edge.to)
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
        let mut path: Vec<Point> = Vec::new();
        for point in std::iter::once(start)
            .chain(middle)
            .chain(std::iter::once(end))
        {
            if path.last() == Some(&point) {
                continue;
            }
            if path.len() >= 2 {
                let (a, b) = (path[path.len() - 2], path[path.len() - 1]);
                // Remove only a straight continuation, preserving any reversal.
                if (a.x == b.x
                    && b.x == point.x
                    && a.y.abs_diff(b.y) + b.y.abs_diff(point.y) == a.y.abs_diff(point.y))
                    || (a.y == b.y
                        && b.y == point.y
                        && a.x.abs_diff(b.x) + b.x.abs_diff(point.x) == a.x.abs_diff(point.x))
                {
                    path.pop();
                }
            }
            path.push(point);
        }
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
