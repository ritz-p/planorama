use super::*;
use crate::layout::metrics::measure;

#[test]
fn large_fixture_limits_each_edges_bends_and_valid_route_stretch() {
    use crate::layout::metrics::measure_edge;
    use std::collections::BTreeMap;
    let raw = plan::parse(include_str!(
        "../../../../examples/terraform-large/plan.json"
    ))
    .unwrap();
    let mut graph = semantic::transform(&raw).0;
    let snapshot = |graph: &Graph| {
        let started = std::time::Instant::now();
        let layout = Layout::new(graph);
        let parallel = crate::layout::routing_shared::spacing::measure(graph, &layout.paths);
        assert!(parallel.close_length <= 4000);
        assert!(layout.width <= 4260 && layout.height <= 2391);
        eprintln!(
            "parallel: {:?}; layout_ms={}",
            crate::layout::routing_shared::spacing::measure(graph, &layout.paths),
            started.elapsed().as_millis()
        );
        let distances: Vec<_> = affinity::groups(graph, &layout.containment.parents)
            .iter()
            .map(|group| {
                let mut centers: Vec<_> = group
                    .sources
                    .iter()
                    .map(|&node| layout.bounds[node].origin.y + layout.bounds[node].height / 2)
                    .collect();
                centers.sort_unstable();
                let median = (centers[(centers.len() - 1) / 2] + centers[centers.len() / 2]) / 2;
                (
                    graph.nodes[group.target].address.as_str(),
                    (layout.bounds[group.target].origin.y + layout.bounds[group.target].height / 2)
                        .abs_diff(median),
                )
            })
            .collect();
        eprintln!(
            "large: {:?}; canvas={}x{} area={}; affinity={distances:?}",
            measure(&layout.paths),
            layout.width,
            layout.height,
            layout.width * layout.height
        );
        assert_no_retraced_stubs(graph, &layout);
        let mut failures = Vec::new();
        let mut result = BTreeMap::new();
        let mut shapes = BTreeMap::new();
        let clearance = crate::layout::routing_shared::spacing::border_metrics(graph, &layout);
        let mut clearances = BTreeMap::new();
        let mut border_total = 0;
        for (index, edge) in graph.edges.iter().enumerate() {
            let path = &layout.paths[index];
            if path.is_empty() {
                continue;
            }
            let reference = routing::shortest_valid_route(graph, &layout, index);
            let metrics = measure_edge(path, &reference);
            eprintln!(
                "shape {} -> {}: {:?}; shortest={}",
                graph.nodes[edge.from].address,
                graph.nodes[edge.to].address,
                crate::layout::routing_shared::spacing::Shape::measure(path),
                metrics.shortest_valid_length
            );
            let from = graph.nodes[edge.from].address.clone();
            let to = graph.nodes[edge.to].address.clone();
            clearances.insert(
                (
                    from.clone(),
                    to.clone(),
                    edge.kind,
                    edge.change.as_ref().map(|c| c.address.clone()),
                ),
                clearance[index],
            );
            if clearance[index].preferred_deficit > 0 {
                eprintln!("border clearance {from} -> {to}: {:?}", clearance[index]);
            }
            let shape = crate::layout::routing_shared::spacing::Shape::measure(path);
            assert_eq!(shape.s_jogs, 0, "{from} -> {to}: {shape:?}");
            if from.contains("aws_subnet.private_")
                && (to.contains("aws_route_table.private")
                    || to.contains("aws_vpc_endpoint.logs")
                    || to.contains("aws_ecs_service.app"))
            {
                use crate::layout::routing_shared::{ports, refinement::terminal, spacing};
                let protected = ports::connect_with_clearances(
                    terminal(path, 0),
                    terminal(path, 1),
                    &spacing::barriers(graph, &layout, index),
                    None,
                    crate::layout::relationship_markers::terminals(
                        graph,
                        index,
                        crate::layout::relationship_markers::numbers(graph)[index],
                        [terminal(path, 0).side, terminal(path, 1).side],
                    ),
                )
                .unwrap();
                let necessary = spacing::Shape::measure(&protected);
                assert_eq!(
                    (
                        shape.offsets,
                        shape.s_jogs,
                        shape.reversals,
                        shape.backtracking
                    ),
                    (0, 0, necessary.reversals, necessary.backtracking),
                    "{from} -> {to}: {shape:?}"
                );
                assert!(
                    shape.bends <= necessary.bends,
                    "{from} -> {to}: path={path:?}; protected={protected:?}"
                );
            }
            shapes.insert(
                (
                    from.clone(),
                    to.clone(),
                    edge.kind,
                    edge.change.as_ref().map(|c| c.address.clone()),
                ),
                shape,
            );
            let hidden = layout.occluders(edge);
            let frames: Vec<_> = layout
                .bounds
                .iter()
                .enumerate()
                .filter(|(node, _)| {
                    !hidden
                        .iter()
                        .any(|&parent| layout.containment.is_ancestor(parent, *node))
                })
                .map(|(_, &b)| b)
                .collect();
            let border = crate::layout::metrics::border_overlap(path, &frames);
            assert_eq!(border, 0, "{from} -> {to}: border overlap={border}");
            border_total += border;
            if border > 0 {
                eprintln!("border {from} -> {to}: {border}");
            }
            if (from.contains("aws_subnet.private_") && to.contains("aws_ecs_service"))
                || (from.contains("aws_internet_gateway") && to.contains("aws_nat_gateway"))
            {
                let own = measure(&[path.clone()]);
                let mut crossings = 0;
                let mut overlap = 0;
                for (other_index, other) in layout.paths.iter().enumerate() {
                    if other_index != index {
                        let pair = measure(&[path.clone(), other.clone()]);
                        crossings += pair.crossing_count;
                        overlap += pair.overlap_distance;
                    }
                }
                eprintln!(
                    "route {from} -> {to}: length={} bends={} crossings={crossings} overlap={overlap} points={path:?}",
                    own.total_path_length, own.bend_count
                );
                if from.contains("aws_subnet.private_") {
                    assert!(own.total_path_length <= 600 && own.bend_count <= 3);
                    assert_eq!(overlap, 0);
                    let source = layout.bounds[edge.from];
                    assert!(
                        path.windows(2)
                            .all(|p| !crate::layout::routing_shared::crosses(p[0], p[1], source)),
                        "source re-entry: {path:?}"
                    );
                } else {
                    let hidden: usize = layout
                        .occluders(edge)
                        .iter()
                        .map(|&node| {
                            let b = layout.bounds[node];
                            path.windows(2)
                                .filter(|p| crate::layout::routing_shared::crosses(p[0], p[1], b))
                                .map(|p| {
                                    if p[0].x == p[1].x {
                                        p[0].y
                                            .max(p[1].y)
                                            .min(b.origin.y + b.height)
                                            .saturating_sub(p[0].y.min(p[1].y).max(b.origin.y))
                                    } else {
                                        p[0].x
                                            .max(p[1].x)
                                            .min(b.right())
                                            .saturating_sub(p[0].x.min(p[1].x).max(b.origin.x))
                                    }
                                })
                                .sum::<usize>()
                        })
                        .sum();
                    eprintln!("gateway hidden length={hidden}");
                    assert!(
                        own.total_path_length <= metrics.shortest_valid_length + 128
                            && own.bend_count <= 3
                    );
                    assert!(hidden as u128 * 3 < own.total_path_length);
                    let foreign = graph
                        .nodes
                        .iter()
                        .position(|n| n.address == "module.network.aws_subnet.public_b")
                        .unwrap();
                    assert!(layout.occludes(foreign, edge));
                }
            }
            if let Some(failure) = metrics.violation(&from, &to, 14, 2, 128) {
                failures.push(failure);
            }
            result.insert(
                (
                    from,
                    to,
                    edge.kind,
                    edge.change.as_ref().map(|c| c.address.clone()),
                ),
                metrics,
            );
        }
        eprintln!("total border overlap={border_total}");
        eprintln!(
            "visible border clearance: minimum_quarter_px={:?}; close_length={}; minimum_deficit={}; preferred_deficit={}",
            clearance.iter().filter_map(|m| m.minimum).min(),
            clearance.iter().map(|m| m.close_length).sum::<usize>(),
            clearance.iter().map(|m| m.minimum_deficit).sum::<u128>(),
            clearance.iter().map(|m| m.preferred_deficit).sum::<u128>(),
        );
        eprintln!(
            "per-edge: count={}, max_bends={}, max_excess={}, max_stretch={:.3}, extreme_routes={}",
            result.len(),
            result.values().map(|m| m.bends).max().unwrap_or(0),
            result
                .values()
                .map(|m| m.excess_distance)
                .max()
                .unwrap_or(0),
            result
                .values()
                .map(|m| m.length as f64 / m.shortest_valid_length.max(1) as f64)
                .fold(0.0, f64::max),
            failures.len()
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        (result, shapes, clearances)
    };
    let original = snapshot(&graph);
    graph.edges.reverse();
    assert_eq!(original, snapshot(&graph));
}

#[test]
fn container_port_stubs_are_not_immediately_retraced() {
    let raw = plan::parse(include_str!("../../../fixtures/spanning-dense-plan.json")).unwrap();
    let graph = semantic::transform(&raw).0;
    let layout = Layout::new(&graph);
    assert_no_retraced_stubs(&graph, &layout);
}

fn assert_no_retraced_stubs(graph: &Graph, layout: &Layout<'_>) {
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        for points in path.windows(3) {
            let [a, b, c] = [points[0], points[1], points[2]];
            let reverses = (a.y == b.y
                && b.y == c.y
                && a.x.cmp(&b.x) != b.x.cmp(&c.x)
                && a.x != b.x
                && b.x != c.x)
                || (a.x == b.x
                    && b.x == c.x
                    && a.y.cmp(&b.y) != b.y.cmp(&c.y)
                    && a.y != b.y
                    && b.y != c.y);
            assert!(
                !reverses,
                "{} -> {} retraces {points:?}",
                graph.nodes[edge.from].address, graph.nodes[edge.to].address
            );
        }
    }
}

#[test]
fn symmetric_peers_keep_ports_and_routes_when_edges_are_reordered() {
    let mut graph = ranked_fixture();
    graph.nodes.truncate(4);
    for (node, module) in [(1, "a"), (2, "b")] {
        graph.nodes[node].address = format!("module.{module}.test.same");
        graph.nodes[node].module = format!("module.{module}");
    }
    graph.edges = (1..4)
        .map(|node| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((0, node))
        })
        .collect();
    graph.edges.extend([Edge::from((1, 3)), Edge::from((2, 3))]);
    let keys = ordering::structural_keys(&graph);
    assert_eq!(keys[1], keys[2]);
    let before = Layout::new(&graph);
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    let after = Layout::new(&reordered);
    assert_eq!(before.bounds, after.bounds);
    assert_eq!(
        before.paths,
        after.paths.into_iter().rev().collect::<Vec<_>>()
    );

    for nested in [true, false] {
        let mut original = graph.clone();
        if !nested {
            original
                .edges
                .retain(|edge| edge.kind != EdgeKind::Containment);
        }
        let baseline = Layout::new(&original);
        let mut reordered = original.clone();
        reordered.nodes.swap(1, 2);
        let remap = |node| match node {
            1 => 2,
            2 => 1,
            other => other,
        };
        for edge in &mut reordered.edges {
            edge.from = remap(edge.from);
            edge.to = remap(edge.to);
        }
        reordered.edges.reverse();
        let result = Layout::new(&reordered);
        for node in 0..original.nodes.len() {
            assert_eq!(baseline.bounds[node], result.bounds[remap(node)]);
        }
        assert_eq!(
            baseline.paths,
            result.paths.into_iter().rev().collect::<Vec<_>>()
        );
    }
}

#[test]
fn coincident_crossings_do_not_justify_a_long_detour() {
    use crate::layout::routing_shared::scoring::Scorer;
    let start = Point { x: 100, y: 2000 };
    let end = Point { x: 500, y: 2000 };
    let obstacles = [
        Bounds {
            origin: Point { x: 200, y: 2440 },
            width: 100,
            height: 100,
        },
        Bounds {
            origin: Point { x: 600, y: 2040 },
            width: 100,
            height: 100,
        },
    ];
    let occupied = vec![Point { x: 350, y: 1600 }, Point { x: 350, y: 2400 }];
    let mut scorer = Scorer::default();
    for _ in 0..8 {
        scorer.insert(occupied.clone());
    }
    let direct = routing::path_between(start, end, &obstacles, None);
    let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
    assert_eq!(measure(&[occupied.clone(), direct]).crossing_count, 1);
    assert_eq!(measure(&[occupied, detour]).crossing_count, 1);
}

#[test]
fn congested_container_routes_improve_over_shortest_paths() {
    let raw = plan::parse(include_str!(
        "../../../../examples/terraform-large/plan.json"
    ))
    .unwrap();
    let architecture = semantic::transform(&raw).0;
    let graph = crate::view::spatial::project(&architecture).graph;
    let layout = Layout::new(&graph);
    let baseline =
        routing::route_with_quality(&graph, &layout, &ordering::structural_keys(&graph), false);
    let old = measure(&baseline);
    let scored =
        routing::route_with_quality(&graph, &layout, &ordering::structural_keys(&graph), true);
    let new = measure(&scored);
    eprintln!("shortest: {old:?}; scored: {new:?}");
    assert!(new.overlap_distance < old.overlap_distance / 3);
    let cost = |m: &crate::layout::metrics::LayoutMetrics| {
        m.overlap_distance * 8
            + m.crossing_count as u128 * crate::layout::routing_shared::scoring::CROSSING_COST
            + m.bend_count as u128 * 96
            + m.total_path_length
    };
    assert!(cost(&new) < cost(&old));
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        let obstacles = routing::obstacles(&layout, edge);
        for pair in path.windows(2) {
            assert!(
                !obstacles
                    .iter()
                    .any(|&b| crate::layout::routing_shared::crosses(pair[0], pair[1], b))
            );
        }
    }
}

#[test]
fn crossings_do_not_force_clear_detours() {
    use crate::layout::routing_shared::scoring::Scorer;
    let start = Point { x: 100, y: 100 };
    let end = Point { x: 500, y: 100 };
    let obstacles = [
        Bounds {
            origin: Point { x: 200, y: 200 },
            width: 100,
            height: 100,
        },
        Bounds {
            origin: Point { x: 600, y: 140 },
            width: 100,
            height: 100,
        },
    ];
    let occupied = vec![Point { x: 350, y: 80 }, Point { x: 350, y: 150 }];
    let shortest = routing::path_between(start, end, &obstacles, None);
    let mut scorer = Scorer::default();
    scorer.insert(occupied.clone());
    let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
    assert_eq!(measure(&[occupied.clone(), shortest]).crossing_count, 1);
    assert_eq!(measure(&[occupied, detour]).crossing_count, 1);
}

#[test]
fn completed_paths_compact_stubs_before_scoring_and_occupancy() {
    use crate::layout::routing_shared::scoring::Scorer;
    for (start, end, expected, split) in [
        (
            Point { x: 100, y: 100 },
            Point { x: 500, y: 220 },
            vec![
                Point { x: 100, y: 100 },
                Point { x: 516, y: 100 },
                Point { x: 516, y: 220 },
                Point { x: 500, y: 220 },
            ],
            Point { x: 116, y: 100 },
        ),
        (
            Point { x: 500, y: 100 },
            Point { x: 100, y: 220 },
            vec![
                Point { x: 500, y: 100 },
                Point { x: 516, y: 100 },
                Point { x: 516, y: 220 },
                Point { x: 100, y: 220 },
            ],
            Point { x: 116, y: 220 },
        ),
    ] {
        for quality in [false, true] {
            let mut scorer = Scorer::default();
            let path = routing::path_between(start, end, &[], quality.then_some(&scorer));
            assert_eq!(path, expected);
            assert_eq!(scorer.readability_cost(&path), 552 + 2 * 96);
            scorer.insert(path);
            let crossing = [
                Point {
                    x: split.x,
                    y: split.y - 10,
                },
                Point {
                    x: split.x,
                    y: split.y + 10,
                },
            ];
            assert_eq!(
                scorer.readability_cost(&crossing),
                20 + crate::layout::routing_shared::scoring::CROSSING_COST
            );
        }
    }
}

#[test]
fn stub_junction_crossings_preserve_short_routes() {
    use crate::layout::routing_shared::scoring::Scorer;
    for (start, end, junction) in [
        (
            Point { x: 100, y: 100 },
            Point { x: 500, y: 220 },
            Point { x: 116, y: 100 },
        ),
        (
            Point { x: 500, y: 100 },
            Point { x: 100, y: 220 },
            Point { x: 116, y: 220 },
        ),
    ] {
        let obstacles = [Bounds {
            origin: Point { x: 207, y: 148 },
            width: 100,
            height: 60,
        }];
        let mut scorer = Scorer::default();
        scorer.insert(vec![
            Point {
                x: junction.x,
                y: junction.y - 10,
            },
            Point {
                x: junction.x,
                y: junction.y + 10,
            },
        ]);
        let direct = routing::path_between(start, end, &obstacles, Some(&Scorer::default()));
        let detour = routing::path_between(start, end, &obstacles, Some(&scorer));
        assert!(
            scorer.readability_cost(&detour) <= scorer.readability_cost(&direct),
            "start={start:?} direct={direct:?} detour={detour:?}"
        );
        assert!(
            scorer.readability_cost(&detour) - Scorer::default().readability_cost(&detour) < 512
        );
    }
}
