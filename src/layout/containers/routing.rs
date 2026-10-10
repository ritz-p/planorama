use super::super::{Bounds, Layout, Point, Port, Side};
use crate::layout::routing_shared::scoring::Scorer;
use crate::model::architecture::{Edge, EdgeKind, Graph};
mod bundles;
mod numbered;
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

pub(in crate::layout) struct Routed {
    pub paths: Vec<Vec<Point>>,
    pub junctions: Vec<Point>,
}

pub(super) fn route(graph: &Graph, layout: &Layout<'_>, keys: &[usize], bundle: bool) -> Routed {
    route_impl(graph, layout, keys, true, bundle, true)
}

pub(in crate::layout) fn route_selected(
    graph: &Graph,
    layout: &Layout<'_>,
    sides: &[[Side; 2]],
) -> Routed {
    route_selected_impl(
        graph,
        layout,
        &layout.containment.keys,
        true,
        true,
        true,
        Some(sides),
    )
}

#[test]
fn refined_ancestor_edges_keep_allocated_sides_and_distinct_ports() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.parent","type":"test"},{"address":"test.child","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.nodes[0].role = crate::model::ResourceRole::Container;
    graph.edges = vec![Edge {
        kind: EdgeKind::Containment,
        ..Edge::from((0, 1))
    }];
    graph.edges.extend((0..3).map(|i| Edge {
        kind: EdgeKind::Association,
        change: Some(crate::model::EdgeChange {
            address: format!("test.link{i}"),
            directionality: crate::model::Directionality::Directed,
            action: crate::model::Action::Create,
            previous_address: None,
            metadata: Default::default(),
        }),
        ..Edge::from((0, 1))
    }));
    let layout = super::place_geometry(&graph, crate::layout::ContainmentTree::new(&graph), true);
    let routed = route_selected(&graph, &layout, &[[Side::Right; 2]; 4]);
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for path in &routed.paths[1..] {
        let start = path[0];
        let end = *path.last().unwrap();
        assert_eq!(start.x, layout.bounds[0].right());
        assert_eq!(end.x, layout.bounds[1].right());
        sources.push(start.y);
        targets.push(end.y);
    }
    for mut offsets in [sources, targets] {
        offsets.sort_unstable();
        assert!(
            offsets
                .windows(2)
                .all(|p| p[1] - p[0] >= crate::layout::routing_shared::slots::MIN_SPACING)
        );
    }
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
    route_selected_impl(graph, layout, keys, quality, bundle, shortcuts, None)
}

fn route_selected_impl(
    graph: &Graph,
    layout: &Layout<'_>,
    keys: &[usize],
    quality: bool,
    bundle: bool,
    shortcuts: bool,
    sides: Option<&[[Side; 2]]>,
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
        for &(edge, source) in edges {
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
    let terminals = numbered::Terminals::with_sides(
        graph,
        layout,
        &order,
        (&source_ports, &target_ports),
        (&source_slots, &target_slots),
        quality,
        sides,
    );
    let reservations = terminals.all();
    let mut scorer = Scorer::default();
    let mut paths = vec![Vec::new(); graph.edges.len()];
    let mut groups = super::affinity::groups(graph, &layout.containment.parents);
    groups.extend(bundles::dependencies(graph, layout));
    let mut attempted = std::collections::BTreeSet::new();
    let mut junctions = Vec::new();
    let mut bundled_edges = vec![false; graph.edges.len()];
    let independent = |index: usize, scorer: &Scorer| {
        let edge = &graph.edges[index];
        let [start, end] = terminals.ports[index];
        let obstacles = terminals.barriers(layout, graph, index);
        let path = if resource_edges[index] || sides.is_some() {
            crate::layout::routing_shared::ports::connect_with_clearances(
                start,
                end,
                &obstacles,
                quality.then_some(scorer),
                terminals.clearances[index],
            )
            .unwrap_or_else(|| panic!("reserved ports have no corridor: edge={index}, start={start:?}, end={end:?}, selected={}", sides.is_some()))
        } else {
            path_with_clearance(
                start.point,
                end.point,
                &obstacles,
                quality.then_some(scorer),
                [16; 2],
            )
        };
        let path = if sides.is_none() && quality && !resource_edges[index] {
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
        if sides.is_none() && !resource_edges[index] {
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
        if sides.is_none() && bundle && !resource_edges[index] {
            if let Some((group_index, group)) = groups.iter().enumerate().find(|(_, group)| {
                group.edges.contains(&index)
                    && group
                        .edges
                        .iter()
                        .all(|&edge| !resource_edges[edge] && paths[edge].is_empty())
            }) {
                if attempted.insert(group_index) {
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
        crate::layout::routing_shared::shortcuts::simplify_routes_with_clearances(
            &mut paths,
            order.into_iter().filter(|&index| !bundled_edges[index]),
            |index| terminals.barriers(layout, graph, index),
            |index| soft_obstacles(layout, &graph.edges[index]),
            |index| terminals.clearances[index],
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
    let numbered = crate::layout::relationship_markers::ends(graph);
    let clearance = crate::layout::relationship_markers::clearance(graph);
    let path = &layout.paths[index];
    if path.len() < 2 {
        return path.clone();
    }
    let start = port(path[0], path[1]);
    let end = port(path[path.len() - 1], path[path.len() - 2]);
    let stubs = [path[1], path[path.len() - 2]]
        .into_iter()
        .zip([start, end])
        .enumerate()
        .map(|(i, (adjacent, p))| {
            if numbered[index][i] {
                clearance
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
            numbered[i].map(|n| if n { clearance } else { 16 }),
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
            scorer
                .choose(vec![shortest, candidate], obstacles)
                .expect("shortest route is within its detour budget")
        }
        None => shortest,
    }
}
