use super::{Bounds, Layout, Point, Scorer, Side, obstacles};
use crate::layout::containers::affinity::Group;
use crate::layout::routing_shared::{Simplification, ports, simplify};
use crate::model::architecture::Graph;

pub(super) struct Bundle {
    pub paths: Vec<(usize, Vec<Point>)>,
    pub junctions: Vec<Point>,
}

pub(super) fn try_bundle(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &Scorer,
    reservations: &[Bounds],
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> Option<Bundle> {
    let target = layout.bounds[group.target];
    let first = *group.sources.first()?;
    let pairs = ports::facing_pairs(layout.bounds[first], target);
    let independent_cost = independent_cost(graph, layout, group, scorer, independent);
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
    scorer: &Scorer,
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> u128 {
    let mut cost = 0;
    let mut occupancy = scorer.clone();
    let mut order = group.edges.clone();
    order.sort_by_key(|&index| {
        let edge = &graph.edges[index];
        let bounds = layout.bounds[edge.from];
        (
            layout.containment.keys[edge.from],
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
            graph.nodes[edge.from].entity.id.as_str(),
        )
    });
    for index in order {
        let edge = &graph.edges[index];
        occupancy.set_peers(
            layout
                .containment
                .routing_peers(edge.from, edge.to, &layout.bounds),
        );
        let path = independent(index, &occupancy);
        cost += occupancy.readability_cost(&path);
        occupancy.insert(path);
    }
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
            .map(|&i| offsets.1[i][target_side as usize])
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
        let start =
            layout.bounds[edge.from].port(source_side, offsets.0[index][source_side as usize]);
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
        let path = simplify(
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
        bundled_cost += scorer.readability_cost(&path);
        coordinates.push(along(start.point));
        paths.push((index, path));
    }
    coordinates.sort_unstable();
    coordinates.dedup();
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
            sources: vec![0, 1],
            target: 2,
            edges: vec![0, 1],
        };
        let scorer = Scorer::default();
        let bundle = test_bundle(
            &graph,
            &layout,
            &group,
            &[[50; 4]; 2],
            &[[50; 4]; 2],
            &scorer,
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
            &scorer,
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
                &scorer,
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
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let scorer = Scorer::default();
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
    let cost = independent_cost(&graph, &layout, &group, &scorer, |index, occupancy| {
        test_independent(
            &graph,
            &layout,
            index,
            (&slots, &slots),
            occupancy,
            &barriers,
        )
    });
    assert!(cost < horizontal_cost, "{cost} >= {horizontal_cost}");
    assert!(test_bundle(&graph, &layout, &group, &slots, &slots, &scorer, &barriers).is_none());
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
    scorer: &Scorer,
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
        sources: vec![0, 1],
        target: 2,
        edges: vec![0, 1],
    };
    let slots = [[50; 4]; 2];
    let scorer = Scorer::default();
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
    let bundle = test_bundle(&graph, &layout, &group, &slots, &slots, &scorer, &barriers).unwrap();
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
        independent_cost(&graph, &layout, &group, &Scorer::default(), independent),
        expected
    );
}
