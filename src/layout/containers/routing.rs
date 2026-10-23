use super::super::{Bounds, Layout, Point, Port, Side};
use crate::layout::routing_shared::scoring::Scorer;
use crate::model::architecture::{Edge, EdgeKind, Graph};
mod bundles;
mod numbered;
use crate::layout::routing_shared::search;
use search::find_path_step;

use crate::layout::routing_shared::{Simplification, simplify};

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

pub(in crate::layout) struct Routed {
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

#[cfg(test)]
pub(super) fn route(graph: &Graph, layout: &Layout<'_>, keys: &[usize], bundle: bool) -> Routed {
    crate::work::complete(route_step(graph, layout, keys, bundle))
}

pub(super) async fn route_step(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    bundle: bool,
) -> Routed {
    crate::work::checkpoint().await;
    route_impl_step(graph, layout, keys, true, bundle, true).await
}

#[cfg(test)]
pub(in crate::layout) fn route_selected(
    graph: &Graph,
    layout: &Layout<'_>,
    sides: &[[Side; 2]],
) -> Routed {
    crate::work::complete(route_selected_step(graph, layout, sides))
}

pub(in crate::layout) async fn route_selected_step(
    graph: &Graph,
    layout: &Layout<'_>,
    sides: &[[Side; 2]],
) -> Routed {
    crate::work::checkpoint().await;
    route_selected_impl_step(
        graph,
        layout,
        &layout.containment.keys,
        true,
        true,
        true,
        Some(sides),
    )
    .await
}

#[cfg(test)]
pub(super) fn route_with_quality(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
) -> Vec<Vec<Point>> {
    route_impl(graph, layout, keys, quality, false, true).paths
}

#[cfg(test)]
fn route_impl(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
    shortcuts: bool,
) -> Routed {
    crate::work::complete(route_impl_step(
        graph, layout, keys, quality, bundle, shortcuts,
    ))
}

async fn route_impl_step(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
    shortcuts: bool,
) -> Routed {
    crate::work::checkpoint().await;
    route_selected_impl_step(graph, layout, keys, quality, bundle, shortcuts, None).await
}

async fn route_selected_impl_step(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
    shortcuts: bool,
    sides: Option<&[[Side; 2]]>,
) -> Routed {
    crate::work::checkpoint().await;
    let geometry = |node: usize| {
        let bounds = layout.bounds[node];
        (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
        )
    };
    let incident = incidents(graph, &layout.containment);
    let resource_edges = crate::layout::resource_edges(graph);
    let numbered = crate::layout::relationship_markers::ends(graph);
    let numbered_targets: Vec<_> = incident
        .iter()
        .map(|edges| {
            edges
                .iter()
                .any(|&(edge, source)| numbered[edge][usize::from(!source)])
        })
        .collect();
    let mut source_ports = vec![0; graph.edges.len()];
    let (source_slots, target_slots) = crate::layout::routing_shared::slots::assign_selected(
        graph,
        &layout.bounds,
        &incident,
        sides,
    );
    let mut target_ports = vec![0; graph.edges.len()];
    for (node, edges) in incident.iter().enumerate() {
        crate::work::checkpoint().await;
        for &(edge, source) in edges {
            crate::work::checkpoint().await;
            let side = sides.map_or(Side::Right, |s| s[edge][usize::from(!source)]);
            let mut port = if source {
                source_slots[edge][side as usize]
            } else {
                target_slots[edge][side as usize]
            };
            if sides.is_none()
                && bundle
                && source
                && graph.edges[edge].kind == EdgeKind::Connection
                && !numbered_targets[node]
            {
                port = 20
                    + port.saturating_sub(20) * layout.header_heights[node].saturating_sub(40)
                        / layout.bounds[node].height.saturating_sub(40).max(1);
            }
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
            !numbered[index][1],
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
    let terminals = numbered::Terminals::with_sides_step(
        graph,
        layout,
        &order,
        (&source_ports, &target_ports),
        (&source_slots, &target_slots),
        quality,
        sides,
    )
    .await;
    let reservations = terminals.all();
    let mut scorer = Scorer::default();
    scorer.set_layout(graph, layout);
    let mut paths = vec![Vec::new(); graph.edges.len()];
    let mut groups = super::affinity::groups(graph, &layout.containment.parents);
    groups.extend(bundles::dependencies(graph, layout));
    let mut attempted = std::collections::BTreeSet::new();
    let mut junctions = Vec::new();
    let mut bundled_edges = vec![false; graph.edges.len()];
    let independent = async |index: usize, scorer: &Scorer| {
        let edge = &graph.edges[index];
        let [start, end] = terminals.ports[index];
        let obstacles = terminals.barriers(layout, graph, index);
        let path = if sides.is_none() && quality && !resource_edges[index] {
            if let Some(path) = crate::layout::routing_shared::ports::select_from_ports_step(
                layout.bounds[edge.from],
                layout.bounds[edge.to],
                [start, end],
                &obstacles,
                scorer,
                (&source_slots[index], &target_slots[index]),
            )
            .await
            {
                path
            } else {
                path_with_clearance_step(start.point, end.point, &obstacles, Some(scorer), [16; 2])
                    .await
            }
        } else if resource_edges[index] || sides.is_some() {
            crate::layout::routing_shared::ports::connect_with_clearances_step(
                start,
                end,
                &obstacles,
                quality.then_some(scorer),
                terminals.clearances[index],
            ).await
            .unwrap_or_else(|| panic!("reserved ports have no corridor: {} -> {}, edge={index}, start={start:?}, end={end:?}, selected={}", graph.nodes[edge.from].address, graph.nodes[edge.to].address, sides.is_some()))
        } else {
            path_with_clearance_step(
                start.point,
                end.point,
                &obstacles,
                quality.then_some(scorer),
                [16; 2],
            )
            .await
        };
        if sides.is_none() && !resource_edges[index] {
            container_endpoint_path_step(layout, edge, path, &obstacles, scorer).await
        } else {
            path
        }
    };
    for &index in &order {
        crate::work::checkpoint().await;
        let edge = &graph.edges[index];
        scorer.set_kind(edge.kind);
        scorer.set_soft(soft_obstacles(layout, edge));
        scorer.set_peers(
            layout
                .containment
                .routing_peers(edge.from, edge.to, &layout.bounds),
        );
        if represented_by_nesting(edge, &layout.containment) {
            continue;
        }
        if !paths[index].is_empty() {
            continue;
        }
        if sides.is_none() && bundle && !resource_edges[index] {
            if let Some((group_index, group)) = groups.iter().enumerate().find(|(_, group)| {
                group.edges.contains(&index)
                    && group
                        .edges
                        .iter()
                        .all(|&edge| !resource_edges[edge] && paths[edge].is_empty())
            }) {
                if attempted.insert(group_index) {
                    if let Some(bundled) = bundles::try_bundle_step(
                        graph,
                        layout,
                        group,
                        (&source_slots, &target_slots),
                        &mut scorer,
                        &reservations,
                        independent,
                    )
                    .await
                    {
                        for (index, path) in bundled.paths {
                            crate::work::checkpoint().await;
                            bundled_edges[index] = true;
                            scorer.insert(path.clone());
                            paths[index] = path;
                        }
                        junctions.extend(bundled.junctions);
                        continue;
                    }
                }
            }
        }
        let path = independent(index, &scorer).await;
        scorer.insert(path.clone());
        paths[index] = path;
    }
    if shortcuts {
        crate::layout::routing_shared::shortcuts::simplify_routes_with_clearances(
            &mut paths,
            order.into_iter().filter(|&index| !bundled_edges[index]),
            |index| terminals.barriers(layout, graph, index),
            |index| soft_obstacles(layout, &graph.edges[index]),
            |index| terminals.clearances[index],
            |index| graph.edges[index].kind,
            scorer,
        );
    }
    junctions.sort_by_key(|point| (point.x, point.y));
    junctions.dedup();
    Routed { paths, junctions }
}

pub(in crate::layout) fn obstacles(layout: &Layout<'_>, edge: &Edge) -> Vec<Bounds> {
    layout
        .bounds
        .iter()
        .enumerate()
        .filter(|&(node, _)| {
            (layout.containers.contains(&node) && !layout.occludes(node, edge))
                || node == edge.from
                || node == edge.to
        })
        .map(|(node, &bounds)| {
            if layout.containers.contains(&node)
                && ((node != edge.from && layout.containment.is_ancestor(node, edge.from))
                    || (node != edge.to && layout.containment.is_ancestor(node, edge.to)))
            {
                bounds.header(layout.header_heights[node])
            } else {
                bounds
            }
        })
        .chain(layout.scopes.iter().map(|panel| panel.bounds.header(36)))
        .collect()
}

pub(in crate::layout) fn soft_obstacles(
    layout: &Layout<'_>,
    edge: &Edge,
) -> Vec<crate::layout::routing_shared::scoring::Occluder> {
    let occluders = layout.occluders(edge);
    layout
        .bounds
        .iter()
        .enumerate()
        .filter_map(|(node, &bounds)| {
            (occluders.contains(&node)
                || (!layout.containers.contains(&node)
                    && node != edge.from
                    && node != edge.to
                    && !occluders
                        .iter()
                        .any(|&parent| layout.containment.is_ancestor(parent, node))))
            .then_some(crate::layout::routing_shared::scoring::Occluder {
                bounds,
                cost: if occluders.contains(&node) { 1 } else { 2 },
                step: if occluders.contains(&node) { 2 } else { 1 },
            })
        })
        .collect()
}

#[cfg(test)]
pub(super) fn shortest_valid_route(graph: &Graph, layout: &Layout<'_>, index: usize) -> Vec<Point> {
    use crate::layout::routing_shared::ports;
    let port = |a: Point, b: Point| Port {
        point: a,
        side: match a.x.cmp(&b.x) {
            std::cmp::Ordering::Equal => {
                if a.y > b.y {
                    Side::Top
                } else {
                    Side::Bottom
                }
            }
            std::cmp::Ordering::Greater => Side::Left,
            std::cmp::Ordering::Less => Side::Right,
        },
    };
    let numbers = crate::layout::relationship_markers::numbers(graph);
    let path = &layout.paths[index];
    if path.len() < 2 {
        return path.clone();
    }
    let start = port(path[0], path[1]);
    let end = port(path[path.len() - 1], path[path.len() - 2]);
    let clearances = crate::layout::relationship_markers::terminals(
        graph,
        index,
        numbers[index],
        [start.side, end.side],
    );
    let stubs = [path[1], path[path.len() - 2]]
        .into_iter()
        .zip([start, end])
        .enumerate()
        .map(|(i, (adjacent, p))| {
            if numbers[index][i].is_some() {
                clearances[i].clearance
            } else {
                16.min(p.point.x.abs_diff(adjacent.x) + p.point.y.abs_diff(adjacent.y))
            }
        })
        .collect::<Vec<_>>();
    let mut barriers = obstacles(layout, &graph.edges[index]);
    for (i, other) in layout.paths.iter().enumerate() {
        if other.len() < 2 {
            continue;
        }
        let a = port(other[0], other[1]);
        let b = port(other[other.len() - 1], other[other.len() - 2]);
        barriers.extend(ports::terminal_corridors(
            a,
            b,
            crate::layout::relationship_markers::terminals(graph, i, numbers[i], [a.side, b.side]),
        ));
    }
    let middle = search::shortest(
        start.outward(stubs[0]),
        end.outward(stubs[1]),
        &barriers,
        (start.side, end.side),
    )
    .expect("rendered route proves that a valid corridor exists");
    let reference = simplify(
        std::iter::once(start.point)
            .chain(middle)
            .chain(std::iter::once(end.point)),
        Simplification::PreserveReversals,
    );
    assert!(reference.windows(2).all(|p| {
        obstacles(layout, &graph.edges[index])
            .iter()
            .all(|&b| !crate::layout::routing_shared::crosses(p[0], p[1], b))
    }));
    reference
}

async fn container_endpoint_path_step(
    layout: &Layout<'_>,
    edge: &Edge,
    mut best: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    crate::work::checkpoint().await;
    for (container, child, reverse) in [(edge.to, edge.from, false), (edge.from, edge.to, true)] {
        crate::work::checkpoint().await;
        if container == child || !layout.containment.is_ancestor(container, child) {
            continue;
        }
        let bounds = layout.bounds[container];
        let node = layout.bounds[child];
        let x = node.origin.x + node.width / 2;
        let y = node.origin.y + node.height / 2;
        let pairs = [
            (
                node.port(Side::Top, node.width / 2),
                Port {
                    point: Point {
                        x,
                        y: bounds.origin.y + layout.header_heights[container],
                    },
                    side: Side::Bottom,
                },
            ),
            (
                node.port(Side::Left, node.height / 2),
                Port {
                    point: Point {
                        x: bounds.origin.x,
                        y,
                    },
                    side: Side::Right,
                },
            ),
            (
                node.port(Side::Right, node.height / 2),
                Port {
                    point: Point {
                        x: bounds.right(),
                        y,
                    },
                    side: Side::Left,
                },
            ),
            (
                node.port(Side::Bottom, node.width / 2),
                Port {
                    point: Point {
                        x,
                        y: bounds.origin.y + bounds.height,
                    },
                    side: Side::Top,
                },
            ),
        ];
        for (a, b) in pairs {
            crate::work::checkpoint().await;
            let (start, end) = if reverse { (b, a) } else { (a, b) };
            if scorer.uses_endpoint(start.point) || scorer.uses_endpoint(end.point) {
                continue;
            }
            if let Some(candidate) = crate::layout::routing_shared::ports::connect_step(
                start,
                end,
                obstacles,
                Some(scorer),
            )
            .await
            {
                if scorer.score(&candidate, obstacles) < scorer.score(&best, obstacles) {
                    best = candidate;
                }
            }
        }
    }
    best
}

#[cfg(test)]
pub(super) fn path_between(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    crate::work::complete(path_between_step(start, end, obstacles, scorer))
}

#[cfg(test)]
pub(super) async fn path_between_step(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Vec<Point> {
    crate::work::checkpoint().await;
    path_with_clearance_step(start, end, obstacles, scorer, [16; 2]).await
}

async fn path_with_clearance_step(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    clearance: [usize; 2],
) -> Vec<Point> {
    crate::work::checkpoint().await;
    let first = Point {
        x: start.x + clearance[0],
        ..start
    };
    let last = Point {
        x: end.x + clearance[1],
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
    let shortest = finish(find_path_step(first, last, obstacles, None).await);
    match scorer {
        Some(scorer) => {
            let candidate = finish(find_path_step(first, last, obstacles, Some(scorer)).await);
            scorer
                .choose(vec![shortest, candidate], obstacles)
                .expect("shortest route is within its detour budget")
        }
        None => shortest,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/containers/routing.rs"]
mod tests;
