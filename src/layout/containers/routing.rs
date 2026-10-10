use super::super::{Bounds, Layout, Point, Port, Side};
use crate::layout::routing_shared::scoring::Scorer;
use crate::model::architecture::{Edge, EdgeKind, Graph};
mod bundles;
use crate::layout::routing_shared::search;
use search::find_path;

use crate::layout::routing_shared::{Simplification, crosses, simplify};

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
    let resource_edges = crate::layout::resource_edges(graph);
    let dense_targets: Vec<_> = incident
        .iter()
        .map(|edges| {
            edges
                .iter()
                .filter(|&&(edge, source)| !source && resource_edges[edge])
                .count()
                > 1
        })
        .collect();
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
        if bundle && !resource_edges[index] {
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
        let start: Port = layout.bounds[edge.from].port(Side::Right, source_ports[index]);
        let end = layout.bounds[edge.to].port(Side::Right, target_ports[index]);
        let obstacles = obstacles(layout, edge);
        let path = path_between(
            start.point,
            end.point,
            &obstacles,
            quality.then_some(&scorer),
        );
        let path = if quality && !(resource_edges[index] && dense_targets[edge.to]) {
            crate::layout::routing_shared::ports::select(
                layout.bounds[edge.from],
                layout.bounds[edge.to],
                path,
                &obstacles,
                &scorer,
            )
        } else {
            path
        };
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
        .chain(layout.scopes.iter().map(|panel| panel.bounds.header(36)))
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
        simplify(
            std::iter::once(start)
                .chain(middle)
                .chain(std::iter::once(end)),
            Simplification::PreserveReversals,
        )
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
