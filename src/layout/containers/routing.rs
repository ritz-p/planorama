use super::super::{Bounds, Layout, Point, Port, Side};
use crate::layout::routing_shared::scoring::Scorer;
use crate::model::architecture::{Edge, EdgeKind, Graph};
mod bundles;
use crate::layout::routing_shared::search;
use search::find_path;

use crate::layout::routing_shared::{Simplification, simplify};

#[test]
fn shortcut_pass_preserves_nested_ports_and_bundles() {
    let raw = crate::plan::parse(include_str!(
        "../../../tests/fixtures/spanning-dense-plan.json"
    ))
    .unwrap();
    let graph = crate::semantic::transform(&raw).0;
    let tree = crate::layout::ContainmentTree::new(&graph);
    let keys = tree.keys.clone();
    let layout = super::place_geometry(&graph, tree, true);
    let before = route_impl(&graph, &layout, &keys, true, true, false);
    let after = route_impl(&graph, &layout, &keys, true, true, true);
    for (a, b) in before.paths.iter().zip(&after.paths) {
        assert_eq!(a.first(), b.first());
        assert_eq!(a.last(), b.last());
        assert!(b.len() <= a.len());
    }
    assert_eq!(before.junctions, after.junctions);
    for (edge, (a, b)) in graph
        .edges
        .iter()
        .zip(before.paths.iter().zip(&after.paths))
    {
        if edge.kind == EdgeKind::Connection {
            assert_eq!(a, b);
        }
    }
    let a = crate::layout::metrics::measure(&before.paths);
    let b = crate::layout::metrics::measure(&after.paths);
    assert!(b.total_path_length <= a.total_path_length);
    assert!(b.overlap_distance <= a.overlap_distance);
    assert!(b.crossing_count <= a.crossing_count);
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
    route_impl(graph, layout, keys, true, bundle, true)
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

fn route_impl(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
    shortcuts: bool,
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
    let numbered = crate::layout::relationship_markers::ends(graph);
    let numbered_targets: Vec<_> = incident
        .iter()
        .map(|edges| {
            edges
                .iter()
                .any(|&(edge, source)| numbered[edge][usize::from(!source)])
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
                layout.bounds[peer].origin.y * 2 + layout.bounds[peer].height,
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
    let (source_slots, target_slots) =
        crate::layout::routing_shared::slots::assign(graph, &layout.bounds, &incident);
    let mut target_ports = vec![0; graph.edges.len()];
    for (node, edges) in incident.iter().enumerate() {
        let ports = crate::layout::relationship_markers::ports(
            edges,
            &numbered,
            layout.bounds[node].height,
        );
        for (slot, &(edge, source)) in edges.iter().enumerate() {
            let height = if bundle
                && source
                && graph.edges[edge].kind == EdgeKind::Connection
                && !numbered_targets[node]
            {
                layout.header_heights[node]
            } else {
                layout.bounds[node].height
            };
            let port = if numbered_targets[node] {
                ports[slot]
            } else {
                20 + (slot + 1) * (height - 40) / (edges.len() + 1)
            };
            match source {
                true => source_ports[edge] = port,
                false => target_ports[edge] = port,
            }
        }
    }
    let clearance = crate::layout::relationship_markers::clearance(graph);
    let spacing = crate::layout::relationship_markers::PORT_SPACING;
    let reservations: Vec<_> = graph
        .edges
        .iter()
        .enumerate()
        .flat_map(|(index, edge)| {
            if !resource_edges[index] || represented_by_nesting(edge, &layout.containment) {
                return Vec::new();
            }
            [
                (edge.from, source_ports[index], numbered[index][0]),
                (edge.to, target_ports[index], numbered[index][1]),
            ]
            .into_iter()
            .filter_map(|(node, offset, enabled)| {
                let end = layout.bounds[node].port(Side::Right, offset).point;
                enabled.then_some(Bounds {
                    origin: Point {
                        x: end.x,
                        y: end.y.saturating_sub(spacing / 2),
                    },
                    width: clearance - 4,
                    height: spacing,
                })
            })
            .collect::<Vec<_>>()
        })
        .collect();
    let reservations = crate::layout::relationship_markers::merge_corridors(reservations);
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
    let mut bundled_edges = vec![false; graph.edges.len()];
    let independent = |index: usize, scorer: &Scorer| {
        let edge = &graph.edges[index];
        let start: Port = layout.bounds[edge.from].port(Side::Right, source_ports[index]);
        let end = layout.bounds[edge.to].port(Side::Right, target_ports[index]);
        let mut obstacles = obstacles(layout, edge);
        obstacles.extend_from_slice(&reservations);
        let path = path_with_clearance(
            start.point,
            end.point,
            &obstacles,
            quality.then_some(scorer),
            [
                if numbered[index][0] { clearance } else { 16 },
                if numbered[index][1] { clearance } else { 16 },
            ],
        );
        let path = if quality && !resource_edges[index] {
            crate::layout::routing_shared::ports::select_with_slots(
                layout.bounds[edge.from],
                layout.bounds[edge.to],
                path,
                &obstacles,
                scorer,
                Some((&source_slots[index], &target_slots[index])),
            )
        } else {
            path
        };
        if !resource_edges[index] {
            container_endpoint_path(layout, edge, path, &obstacles, scorer)
        } else {
            path
        }
    };
    for &index in &order {
        let edge = &graph.edges[index];
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
        if bundle && !resource_edges[index] {
            if let Some(group) = groups.iter().find(|group| {
                group.edges.contains(&index)
                    && group.edges.iter().all(|&edge| !resource_edges[edge])
            }) {
                if attempted.insert(group.target) {
                    if let Some(bundled) = bundles::try_bundle(
                        graph,
                        layout,
                        group,
                        (&source_slots, &target_slots),
                        &mut scorer,
                        &reservations,
                        independent,
                    ) {
                        for (index, path) in bundled.paths {
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
        let path = independent(index, &scorer);
        scorer.insert(path.clone());
        paths[index] = path;
    }
    if shortcuts {
        crate::layout::routing_shared::shortcuts::simplify_routes_with_soft(
            &mut paths,
            order
                .into_iter()
                .filter(|&index| !resource_edges[index] && !bundled_edges[index]),
            |index| {
                let mut barriers = obstacles(layout, &graph.edges[index]);
                barriers.extend_from_slice(&reservations);
                barriers
            },
            |index| soft_obstacles(layout, &graph.edges[index]),
        );
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
        .filter(|&(node, _)| {
            layout.containers.contains(&node) || node == edge.from || node == edge.to
        })
        .map(|(node, &bounds)| {
            if layout.containers.contains(&node)
                && (layout.containment.is_ancestor(node, edge.from)
                    || layout.containment.is_ancestor(node, edge.to))
            {
                bounds.header(layout.header_heights[node])
            } else {
                bounds
            }
        })
        .chain(layout.scopes.iter().map(|panel| panel.bounds.header(36)))
        .collect()
}

pub(super) fn soft_obstacles(layout: &Layout<'_>, edge: &Edge) -> Vec<Bounds> {
    layout
        .bounds
        .iter()
        .enumerate()
        .filter_map(|(node, &bounds)| {
            (!layout.containers.contains(&node) && node != edge.from && node != edge.to)
                .then_some(bounds)
        })
        .collect()
}

fn container_endpoint_path(
    layout: &Layout<'_>,
    edge: &Edge,
    mut best: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    for (container, child, reverse) in [(edge.to, edge.from, false), (edge.from, edge.to, true)] {
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
            let (start, end) = if reverse { (b, a) } else { (a, b) };
            if scorer.uses_endpoint(start.point) || scorer.uses_endpoint(end.point) {
                continue;
            }
            if let Some(candidate) =
                crate::layout::routing_shared::ports::connect(start, end, obstacles, Some(scorer))
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
    path_with_clearance(start, end, obstacles, scorer, [16; 2])
}

fn path_with_clearance(
    start: Point,
    end: Point,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    clearance: [usize; 2],
) -> Vec<Point> {
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
