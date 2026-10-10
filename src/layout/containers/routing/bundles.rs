use super::{Bounds, Layout, Point, Scorer, Side, obstacles};
use crate::layout::containers::affinity::Group;
use crate::layout::routing_shared::{Simplification, ports, simplify};
use crate::model::architecture::{EdgeKind, Graph};

pub(super) fn dependencies(graph: &Graph, layout: &Layout<'_>) -> Vec<Group> {
    use std::collections::{BTreeMap, BTreeSet};
    let numbered = crate::layout::resource_edges(graph);
    let mut buckets: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for reverse in [true, false] {
        for (index, edge) in graph.edges.iter().enumerate() {
            if edge.kind != EdgeKind::Dependency
                || edge.change.is_some()
                || numbered[index]
                || edge.from == edge.to
                || super::represented_by_nesting(edge, &layout.containment)
            {
                continue;
            }
            let (shared, peer) = if reverse {
                (edge.from, edge.to)
            } else {
                (edge.to, edge.from)
            };
            buckets
                .entry((reverse, shared, layout.containment.parents[peer]))
                .or_default()
                .push(index);
        }
    }
    buckets
        .into_iter()
        .filter_map(|((reverse, target, _), mut edges)| {
            edges.sort_by_key(|&i| {
                let e = &graph.edges[i];
                (
                    graph.nodes[e.from].entity.id.as_str(),
                    graph.nodes[e.to].entity.id.as_str(),
                )
            });
            let sources: BTreeSet<_> = edges
                .iter()
                .map(|&i| {
                    if reverse {
                        graph.edges[i].to
                    } else {
                        graph.edges[i].from
                    }
                })
                .collect();
            (sources.len() > 1 && sources.len() == edges.len()).then(|| Group {
                reverse,
                target,
                sources: sources.into_iter().collect(),
                edges,
            })
        })
        .collect()
}

pub(super) struct Bundle {
    pub paths: Vec<(usize, Vec<Point>)>,
    pub junctions: Vec<Point>,
}

pub(super) fn try_bundle(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &mut Scorer,
    reservations: &[Bounds],
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> Option<Bundle> {
    let target = layout.bounds[group.target];
    let first = *group.sources.first()?;
    let pairs = ports::facing_pairs(layout.bounds[first], target);
    let mut independent_cost = None;
    let mut best = None;
    for pair in pairs {
        if !group
            .sources
            .iter()
            .all(|&source| ports::facing_pairs(layout.bounds[source], target).contains(&pair))
        {
            continue;
        }
        for clearance in [16, 24, 8] {
            if let Some((bundle, cost)) = candidate(
                graph,
                layout,
                group,
                offsets,
                scorer,
                reservations,
                (pair, clearance),
            ) {
                let independent_cost = *independent_cost.get_or_insert_with(|| {
                    self::independent_cost(graph, layout, group, scorer, reservations, &independent)
                });
                if cost <= independent_cost && best.as_ref().is_none_or(|(_, old)| cost < *old) {
                    best = Some((bundle, cost));
                }
            }
        }
    }
    best.map(|(bundle, _)| bundle)
}

fn independent_cost(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    scorer: &mut Scorer,
    reservations: &[Bounds],
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> u128 {
    let mut cost = 0;
    let checkpoint = scorer.len();
    let peers = scorer.peers().to_vec();
    let soft = scorer.soft().to_vec();
    let mut paths = Vec::new();
    let mut order = group.edges.clone();
    order.sort_by_key(|&index| {
        let edge = &graph.edges[index];
        let bounds = layout.bounds[edge.from];
        (
            layout.containment.keys[edge.from],
            layout.containment.keys[edge.to],
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
            graph.nodes[edge.from].entity.id.as_str(),
            graph.nodes[edge.to].entity.id.as_str(),
        )
    });
    for &index in &order {
        let edge = &graph.edges[index];
        scorer.set_soft(super::soft_obstacles(layout, edge));
        scorer.set_peers(
            layout
                .containment
                .routing_peers(edge.from, edge.to, &layout.bounds),
        );
        let path = independent(index, scorer);
        scorer.insert(path.clone());
        paths.push(path);
    }
    for (slot, &index) in order.iter().enumerate() {
        scorer.set_soft(super::soft_obstacles(layout, &graph.edges[index]));
        let mut barriers = obstacles(layout, &graph.edges[index]);
        barriers.extend_from_slice(reservations);
        scorer.remove(checkpoint + slot);
        paths[slot] = crate::layout::routing_shared::shortcuts::simplify_path(
            std::mem::take(&mut paths[slot]),
            &barriers,
            scorer,
        );
        scorer.replace(checkpoint + slot, paths[slot].clone());
    }
    scorer.truncate(checkpoint);
    for (index, path) in order.into_iter().zip(paths) {
        scorer.set_soft(super::soft_obstacles(layout, &graph.edges[index]));
        cost += scorer.readability_cost(&path);
        scorer.insert(path);
    }
    scorer.truncate(checkpoint);
    scorer.set_peers(peers);
    scorer.set_soft(soft);
    cost
}

fn candidate(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &Scorer,
    reservations: &[Bounds],
    choice: ((Side, Side), usize),
) -> Option<(Bundle, u128)> {
    let ((source_side, target_side), clearance) = choice;
    let target = layout.bounds[group.target];
    let end = target.port(
        target_side,
        group
            .edges
            .iter()
            .map(|&i| {
                if group.reverse {
                    offsets.0[i][target_side as usize]
                } else {
                    offsets.1[i][target_side as usize]
                }
            })
            .min()?,
    );
    let trunk = end.outward(clearance);
    if trunk == end.point {
        return None;
    }
    let horizontal = matches!(source_side, Side::Left | Side::Right);
    let along = |p: Point| if horizontal { p.y } else { p.x };
    let mut paths = Vec::new();
    let mut coordinates = vec![along(end.point)];
    let mut bundled_cost = 0;
    for &index in &group.edges {
        let edge = &graph.edges[index];
        let node = if group.reverse { edge.to } else { edge.from };
        let slots = if group.reverse { offsets.1 } else { offsets.0 };
        let start = layout.bounds[node].port(source_side, slots[index][source_side as usize]);
        let branch = if horizontal {
            Point {
                x: trunk.x,
                y: start.point.y,
            }
        } else {
            Point {
                x: start.point.x,
                y: trunk.y,
            }
        };
        let mut path = simplify(
            [start.point, branch, trunk, end.point],
            Simplification::PreserveReversals,
        );
        let mut barriers = obstacles(layout, edge);
        barriers.extend_from_slice(reservations);
        if !ports::valid_with_clearance(&path, start, end, &barriers, clearance.min(16))
            || scorer.overlaps(&path)
        {
            return None;
        }
        if group.reverse {
            path.reverse();
        }
        bundled_cost +=
            scorer.readability_cost_with_soft(&path, &super::soft_obstacles(layout, edge));
        coordinates.push(along(start.point));
        paths.push((index, path));
    }
    coordinates.sort_unstable();
    coordinates.dedup();
    if group
        .edges
        .iter()
        .all(|&i| graph.edges[i].kind == EdgeKind::Dependency)
    {
        bundled_cost = bundled_cost.saturating_sub(
            (group.edges.len() - 1) as u128 * crate::layout::routing_shared::scoring::BEND_COST,
        );
    }
    let junctions = coordinates
        .iter()
        .skip(1)
        .take(coordinates.len().saturating_sub(2))
        .map(|&coordinate| {
            if horizontal {
                Point {
                    x: trunk.x,
                    y: coordinate,
                }
            } else {
                Point {
                    x: coordinate,
                    y: trunk.y,
                }
            }
        })
        .collect();
    Some((Bundle { paths, junctions }, bundled_cost))
}

#[test]
fn dependency_fan_in_and_out_share_safe_nested_corridors() {
    use crate::layout::Point;
    use crate::model::architecture::{Edge, ResourceRole};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.hub","type":"test"},{"address":"test.vpc","type":"test"},{"address":"test.subnet","type":"test"}]}"#).unwrap();
    for reverse in [false, true] {
        let mut graph = crate::semantic::transform(&raw).0;
        graph.nodes[3].role = ResourceRole::Container;
        graph.nodes[4].role = ResourceRole::Container;
        graph.edges = [(0, 2), (1, 2)]
            .map(|(from, to)| Edge::from(if reverse { (to, from) } else { (from, to) }))
            .into();
        graph
            .edges
            .extend([(3, 4), (4, 0), (4, 1), (4, 2)].map(|p| Edge {
                kind: EdgeKind::Containment,
                ..Edge::from(p)
            }));
        let mut layout = Layout::new(&graph);
        layout.bounds = [
            (100, 180, 100, 100),
            (100, 380, 100, 100),
            (500, 280, 100, 100),
            (20, 20, 800, 650),
            (60, 100, 700, 500),
        ]
        .map(|(x, y, width, height)| Bounds {
            origin: Point { x, y },
            width,
            height,
        })
        .into();
        layout.header_heights = vec![100, 100, 100, 40, 40];
        let before = graph.clone();
        let groups = dependencies(&graph, &layout);
        let group = groups
            .iter()
            .find(|g| g.reverse == reverse && g.target == 2)
            .unwrap();
        let slots = vec![[50; 4]; graph.edges.len()];
        let bundle = test_bundle(
            &graph,
            &layout,
            group,
            &slots,
            &slots,
            &mut Scorer::default(),
            &[],
        )
        .unwrap();
        assert!(!bundle.junctions.is_empty());
        let routed = super::route(&graph, &layout, &layout.containment.keys, true);
        assert!(!routed.junctions.is_empty());
        layout.paths = routed.paths;
        layout.junctions = routed.junctions;
        let svg = crate::svg::render(&graph, &layout);
        assert_eq!(svg.matches("data-edge-kind=\"dependency\"").count(), 2);
        assert_eq!(svg.matches("marker-end=").count(), 2);
        for (index, path) in &bundle.paths {
            let edge = &graph.edges[*index];
            let from = layout.bounds[edge.from];
            let to = layout.bounds[edge.to];
            let pair = ports::facing_pairs(from, to)[0];
            assert_eq!(path.first(), Some(&from.port(pair.0, 50).point));
            assert_eq!(path.last(), Some(&to.port(pair.1, 50).point));
            assert!(path.windows(2).all(|p| {
                obstacles(&layout, edge)
                    .iter()
                    .all(|&b| !crate::layout::routing_shared::crosses(p[0], p[1], b))
            }));
        }
        assert_eq!(graph, before);
        let blocked = [Bounds {
            origin: Point { x: 472, y: 310 },
            width: 24,
            height: 40,
        }];
        assert!(
            test_bundle(
                &graph,
                &layout,
                group,
                &slots,
                &slots,
                &mut Scorer::default(),
                &blocked
            )
            .is_none()
        );
        let mut reordered = graph.clone();
        reordered.edges.reverse();
        let again_groups = dependencies(&reordered, &layout);
        let again_group = again_groups
            .iter()
            .find(|g| g.reverse == reverse && g.target == 2)
            .unwrap();
        let again = test_bundle(
            &reordered,
            &layout,
            again_group,
            &slots,
            &slots,
            &mut Scorer::default(),
            &[],
        )
        .unwrap();
        assert_eq!(bundle.junctions, again.junctions);
        for (edge, path) in &bundle.paths {
            let index = graph.edges.len() - 1 - edge;
            assert_eq!(
                path,
                &again.paths.iter().find(|(i, _)| *i == index).unwrap().1
            );
        }
        layout.containment.parents[1] = Some(3);
        assert!(
            dependencies(&graph, &layout)
                .iter()
                .all(|g| g.target != 2 || g.reverse != reverse)
        );
    }
}

#[test]
fn bundles_use_all_facing_directions_and_fall_back_when_blocked() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    for (mirror, vertical) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut layout = Layout::new(&graph);
        layout.bounds = [(100, 100), (100, 300), (500, 200)]
            .map(|(mut x, mut y)| {
                if mirror {
                    x = 800 - x - 100;
                }
                if vertical {
                    std::mem::swap(&mut x, &mut y);
                }
                Bounds {
                    origin: Point { x, y },
                    width: 100,
                    height: 100,
                }
            })
            .into();
        layout.header_heights = vec![100; 3];
        let group = Group {
            reverse: false,
            sources: vec![0, 1],
            target: 2,
            edges: vec![0, 1],
        };
        let mut scorer = Scorer::default();
        let bundle = test_bundle(
            &graph,
            &layout,
            &group,
            &[[50; 4]; 2],
            &[[50; 4]; 2],
            &mut scorer,
            &[],
        )
        .unwrap();
        assert!(!bundle.junctions.is_empty());
        let pair = ports::facing_pairs(layout.bounds[0], layout.bounds[2])[0];
        for (index, path) in &bundle.paths {
            assert_eq!(
                path.first(),
                Some(
                    &layout.bounds[graph.edges[*index].from]
                        .port(pair.0, 50)
                        .point
                )
            );
            assert_eq!(path.last(), Some(&layout.bounds[2].port(pair.1, 50).point));
        }
        let mut reversed = graph.clone();
        reversed.edges.reverse();
        let again = test_bundle(
            &reversed,
            &layout,
            &group,
            &[[50; 4]; 2],
            &[[50; 4]; 2],
            &mut scorer,
            &[],
        )
        .unwrap();
        assert_eq!(bundle.junctions, again.junctions);
        assert_eq!(bundle.paths[0].1, again.paths[1].1);
        let end = layout.bounds[2].port(pair.1, 50).outward(8);
        let blocker = Bounds {
            origin: Point {
                x: end.x - 4,
                y: end.y - 4,
            },
            width: 8,
            height: 8,
        };
        assert!(
            test_bundle(
                &graph,
                &layout,
                &group,
                &[[50; 4]; 2],
                &[[50; 4]; 2],
                &mut scorer,
                &[blocker]
            )
            .is_none()
        );
    }
}

#[test]
fn vertical_bundles_use_projected_slots_with_other_incident_edges() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"},{"address":"test.other","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2), (0, 3), (2, 3)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    let mut layout = Layout::new(&graph);
    layout.bounds = [
        (100, 100, 180, 100),
        (350, 100, 180, 100),
        (500, 500, 240, 120),
        (700, 50, 100, 100),
    ]
    .map(|(x, y, width, height)| Bounds {
        origin: Point { x, y },
        width,
        height,
    })
    .into();
    let incident = super::incidents(&graph, &layout.containment);
    let (sources, targets) =
        crate::layout::routing_shared::slots::assign(&graph, &layout.bounds, &incident);
    let group = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let scorer = Scorer::default();
    let (bundle, _) = candidate(
        &graph,
        &layout,
        &group,
        (&sources, &targets),
        &scorer,
        &[],
        ((Side::Bottom, Side::Top), 16),
    )
    .unwrap();
    let offset = targets[0][Side::Top as usize].min(targets[1][Side::Top as usize]);
    for (index, path) in &bundle.paths {
        let source = graph.edges[*index].from;
        assert_eq!(
            path.first(),
            Some(
                &layout.bounds[source]
                    .port(Side::Bottom, sources[*index][Side::Bottom as usize])
                    .point
            )
        );
        assert_eq!(
            path.last(),
            Some(&layout.bounds[2].port(Side::Top, offset).point)
        );
    }
    assert!(sources[0][Side::Bottom as usize] < sources[2][Side::Bottom as usize]);
    assert!(targets[0][Side::Top as usize] < targets[1][Side::Top as usize]);
    assert_ne!(
        sources[0][Side::Bottom as usize],
        sources[0][Side::Right as usize] * layout.bounds[0].width / layout.bounds[0].height
    );
    let mut graph = graph.clone();
    graph.edges.reverse();
    let (sources, targets) = crate::layout::routing_shared::slots::assign(
        &graph,
        &layout.bounds,
        &super::incidents(&graph, &layout.containment),
    );
    let reordered = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![2, 3],
    };
    let (again, _) = candidate(
        &graph,
        &layout,
        &reordered,
        (&sources, &targets),
        &scorer,
        &[],
        ((Side::Bottom, Side::Top), 16),
    )
    .unwrap();
    assert_eq!(bundle.paths[0].1, again.paths[1].1);
    assert_eq!(bundle.junctions, again.junctions);
}

#[test]
fn blocked_vertical_trunk_does_not_hide_cheaper_vertical_independent_routes() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    let mut layout = Layout::new(&graph);
    layout.bounds = [(100, 100), (100, 350), (500, 700)]
        .map(|(x, y)| Bounds {
            origin: Point { x, y },
            width: 100,
            height: 200,
        })
        .into();
    let slots = [[100, 100, 50, 50]; 2];
    let group = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let mut scorer = Scorer::default();
    let barriers = [Bounds {
        origin: Point { x: 300, y: 660 },
        width: 100,
        height: 40,
    }];
    let (_, horizontal_cost) = candidate(
        &graph,
        &layout,
        &group,
        (&slots, &slots),
        &scorer,
        &barriers,
        ((Side::Right, Side::Left), 16),
    )
    .unwrap();
    for clearance in [8, 16, 24] {
        assert!(
            candidate(
                &graph,
                &layout,
                &group,
                (&slots, &slots),
                &scorer,
                &barriers,
                ((Side::Bottom, Side::Top), clearance)
            )
            .is_none()
        );
    }
    let cost = independent_cost(
        &graph,
        &layout,
        &group,
        &mut scorer,
        &barriers,
        |index, occupancy| {
            test_independent(
                &graph,
                &layout,
                index,
                (&slots, &slots),
                occupancy,
                &barriers,
            )
        },
    );
    assert!(cost < horizontal_cost, "{cost} >= {horizontal_cost}");
    assert!(
        test_bundle(
            &graph,
            &layout,
            &group,
            &slots,
            &slots,
            &mut scorer,
            &barriers
        )
        .is_none()
    );
}

#[cfg(test)]
fn test_independent(
    graph: &Graph,
    layout: &Layout<'_>,
    index: usize,
    slots: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &Scorer,
    reservations: &[Bounds],
) -> Vec<Point> {
    let edge = &graph.edges[index];
    let source = layout.bounds[edge.from];
    let target = layout.bounds[edge.to];
    let mut barriers = obstacles(layout, edge);
    barriers.extend_from_slice(reservations);
    let baseline = super::path_between(
        source
            .port(Side::Right, slots.0[index][Side::Right as usize])
            .point,
        target
            .port(Side::Right, slots.1[index][Side::Right as usize])
            .point,
        &barriers,
        Some(scorer),
    );
    ports::select_with_slots(
        source,
        target,
        baseline,
        &barriers,
        scorer,
        Some((&slots.0[index], &slots.1[index])),
    )
}

#[cfg(test)]
fn test_bundle(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    sources: &[[usize; 4]],
    targets: &[[usize; 4]],
    scorer: &mut Scorer,
    reservations: &[Bounds],
) -> Option<Bundle> {
    try_bundle(
        graph,
        layout,
        group,
        (sources, targets),
        scorer,
        reservations,
        |index, occupancy| {
            test_independent(
                graph,
                layout,
                index,
                (sources, targets),
                occupancy,
                reservations,
            )
        },
    )
}

#[test]
fn narrow_corridor_retains_eight_pixel_bundle() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    let mut layout = Layout::new(&graph);
    layout.bounds = [(100, 100), (100, 300), (500, 200)]
        .map(|(x, y)| Bounds {
            origin: Point { x, y },
            width: 100,
            height: 100,
        })
        .into();
    let group = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let slots = [[50; 4]; 2];
    let mut scorer = Scorer::default();
    let barriers = [Bounds {
        origin: Point { x: 472, y: 210 },
        width: 16,
        height: 20,
    }];
    for clearance in [16, 24] {
        assert!(
            candidate(
                &graph,
                &layout,
                &group,
                (&slots, &slots),
                &scorer,
                &barriers,
                ((Side::Right, Side::Left), clearance)
            )
            .is_none()
        );
    }
    let bundle = test_bundle(
        &graph,
        &layout,
        &group,
        &slots,
        &slots,
        &mut scorer,
        &barriers,
    )
    .unwrap();
    for (_, path) in bundle.paths {
        assert_eq!(path[path.len() - 2].x, 492);
        assert_eq!(path.last().unwrap().x, 500);
        assert!(path.windows(2).all(|p| {
            barriers
                .iter()
                .all(|&b| !crate::layout::routing_shared::crosses(p[0], p[1], b))
        }));
    }
}

#[test]
fn comparison_preserves_header_ports_when_facing_slots_are_blocked() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    let mut layout = Layout::new(&graph);
    layout.bounds = [(100, 100), (100, 400), (500, 200)]
        .map(|(x, y)| Bounds {
            origin: Point { x, y },
            width: 100,
            height: 200,
        })
        .into();
    let group = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let slots = [[100, 100, 50, 50]; 2];
    let barriers = [195, 495].map(|y| Bounds {
        origin: Point { x: 205, y },
        width: 10,
        height: 10,
    });
    let independent = |index: usize, scorer: &Scorer| {
        let edge = &graph.edges[index];
        let source = layout.bounds[edge.from];
        let target = layout.bounds[edge.to];
        let mut obstacles = obstacles(&layout, edge);
        obstacles.extend_from_slice(&barriers);
        let start = source.port(Side::Right, 30).point;
        let end = target.port(Side::Right, 80 + index * 40).point;
        let baseline = super::path_between(start, end, &obstacles, Some(scorer));
        let path = ports::select_with_slots(
            source,
            target,
            baseline,
            &obstacles,
            scorer,
            Some((&slots[index], &slots[index])),
        );
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&end));
        path
    };
    let mut occupancy = Scorer::default();
    let mut expected = 0;
    for index in 0..2 {
        let path = independent(index, &occupancy);
        expected += occupancy.readability_cost(&path);
        occupancy.insert(path);
    }
    assert_eq!(
        independent_cost(
            &graph,
            &layout,
            &group,
            &mut Scorer::default(),
            &barriers,
            independent
        ),
        expected
    );
}

#[test]
fn shortened_independent_paths_beat_a_bundle_and_restore_occupancy() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    let mut layout = Layout::new(&graph);
    layout.bounds = [(100, 100), (100, 300), (500, 200)]
        .map(|(x, y)| Bounds {
            origin: Point { x, y },
            width: 100,
            height: 100,
        })
        .into();
    let group = Group {
        reverse: false,
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let sources = [[50; 4]; 2];
    let targets = [[30; 4], [70; 4]];
    let paths = [
        [
            (200, 150),
            (240, 150),
            (240, 180),
            (300, 180),
            (300, 230),
            (500, 230),
        ],
        [
            (200, 350),
            (240, 350),
            (240, 320),
            (300, 320),
            (300, 270),
            (500, 270),
        ],
    ]
    .map(|path| path.map(|(x, y)| Point { x, y }).to_vec());
    let mut scorer = Scorer::default();
    for x in 1000..3000 {
        scorer.insert(vec![Point { x, y: 1000 }, Point { x, y: 1100 }]);
    }
    let peers = vec![layout.bounds[0]];
    scorer.set_peers(peers.clone());
    let query = [Point { x: 900, y: 1050 }, Point { x: 3100, y: 1050 }];
    let before = scorer.score(&query, &[]);
    let (_, bundle_cost) = candidate(
        &graph,
        &layout,
        &group,
        (&sources, &targets),
        &scorer,
        &[],
        ((Side::Right, Side::Left), 16),
    )
    .unwrap();
    let raw_cost: u128 = paths.iter().map(|path| scorer.readability_cost(path)).sum();
    assert!(bundle_cost < raw_cost);
    for _ in 0..100 {
        let cost = independent_cost(&graph, &layout, &group, &mut scorer, &[], |index, _| {
            paths[index].clone()
        });
        assert!(cost < bundle_cost, "{cost} >= {bundle_cost}");
        assert_eq!(scorer.len(), 2000);
        assert_eq!(scorer.peers(), peers);
        assert_eq!(scorer.score(&query, &[]), before);
        assert!(!scorer.uses_endpoint(paths[0][0]));
        assert!(!scorer.overlaps(&paths[0]));
    }
    assert!(
        try_bundle(
            &graph,
            &layout,
            &group,
            (&sources, &targets),
            &mut scorer,
            &[],
            |index, _| paths[index].clone()
        )
        .is_none()
    );
}
