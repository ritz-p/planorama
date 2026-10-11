use super::*;

#[test]
fn numbered_parallel_and_mixed_slots_are_spaced_and_stable() {
    use crate::layout::Point;
    for fixture in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/aws-relationships-plan.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/association-plan.json"
        )),
    ] {
        let mut graph = crate::semantic::transform(&crate::plan::parse(fixture).unwrap()).0;
        let edge = graph
            .edges
            .iter()
            .find(|e| e.change.is_some())
            .unwrap()
            .clone();
        if graph.edges.len() == 1 {
            let mut parallel = edge.clone();
            parallel
                .change
                .as_mut()
                .unwrap()
                .address
                .push_str("_parallel");
            graph.edges.push(parallel);
        }
        graph
            .edges
            .push(crate::model::Edge::from((edge.from, edge.to)));
        let bounds = vec![
            Bounds {
                origin: Point { x: 100, y: 100 },
                width: 400,
                height: 300
            };
            graph.nodes.len()
        ];
        let snapshot = |graph: &Graph| {
            let mut incident = vec![Vec::new(); graph.nodes.len()];
            for (i, edge) in graph.edges.iter().enumerate() {
                incident[edge.from].push((i, true));
                incident[edge.to].push((i, false));
            }
            let (sources, targets) = assign(graph, &bounds, &incident);
            let numbered = crate::layout::relationship_markers::ends(graph);
            for edges in incident {
                for side in 0..4 {
                    let mut offsets: Vec<_> = edges
                        .iter()
                        .map(|&(e, source)| {
                            (
                                if source {
                                    sources[e][side]
                                } else {
                                    targets[e][side]
                                },
                                numbered[e][usize::from(!source)],
                            )
                        })
                        .collect();
                    offsets.sort();
                    for pair in offsets.windows(2) {
                        assert!(pair[0].0 < pair[1].0);
                        if pair[0].1 && pair[1].1 {
                            assert!(pair[1].0 - pair[0].0 >= 24);
                        }
                    }
                }
            }
            (sources, targets)
        };
        let original = snapshot(&graph);
        graph.edges.reverse();
        let reversed = snapshot(&graph);
        assert_eq!(original.0, reversed.0.into_iter().rev().collect::<Vec<_>>());
        assert_eq!(original.1, reversed.1.into_iter().rev().collect::<Vec<_>>());
    }
}

#[test]
fn horizontal_demand_uses_the_denser_side_and_marker_widths() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.peer","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![crate::model::Edge::from((0, 1)); 8];
    for side in [Side::Left, Side::Right] {
        let sides = vec![[side; 2]; 8];
        assert_eq!(required_heights(&graph, &sides, &[true; 8]), vec![169; 2]);
    }
    let sides: Vec<_> = (0..8)
        .map(|i| [if i < 4 { Side::Left } else { Side::Right }; 2])
        .collect();
    assert_eq!(required_heights(&graph, &sides, &[true; 8]), vec![105; 2]);
    assert_eq!(
        required_heights(&graph, &[[Side::Top; 2]; 8], &[true; 8]),
        vec![41; 2]
    );

    let mut graph = crate::semantic::transform(
        &crate::plan::parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/association-plan.json"
        )))
        .unwrap(),
    )
    .0;
    let edge = graph
        .edges
        .iter()
        .find(|e| e.change.is_some())
        .unwrap()
        .clone();
    graph.edges = vec![edge.clone(); 4];
    let heights = required_heights(&graph, &[[Side::Right; 2]; 4], &[true; 4]);
    assert_eq!(heights[edge.from], 137);
    assert_eq!(heights[edge.to], 137);
    graph
        .edges
        .push(crate::model::Edge::from((edge.from, edge.to)));
    let heights = required_heights(&graph, &[[Side::Right; 2]; 5], &[true; 5]);
    assert_eq!(heights[edge.from], 153);
    assert_eq!(heights[edge.to], 153);
}

#[test]
fn coincident_ports_are_scoped_to_nodes_and_distinct_opposite_roles() {
    use crate::layout::Point;
    use crate::model::Edge;
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.c","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    let point = Point { x: 100, y: 100 };
    let paths = vec![
        vec![point, Point { x: 200, y: 100 }],
        vec![Point { x: 50, y: 100 }, point],
    ];
    graph.edges = vec![Edge::from((0, 1)), Edge::from((2, 0))];
    assert!(opposite_roles_collide(&graph, &paths));
    for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        assert!(crowded(&graph, &paths, &[[side; 2]; 2]));
    }
    graph.edges[1].to = 1;
    assert!(!opposite_roles_collide(&graph, &paths));
    graph.edges[1] = Edge::from((0, 2));
    let shared = vec![paths[0].clone(), vec![point, Point { x: 200, y: 200 }]];
    assert!(!opposite_roles_collide(&graph, &shared));
}

#[test]
fn overfull_horizontal_ports_move_to_sides_before_allocation_clamps() {
    use crate::model::Edge;
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = (0..24)
        .map(|i| Edge::from(if i % 2 == 0 { (0, 1) } else { (1, 0) }))
        .collect();
    let bounds = vec![
        Bounds::card(crate::layout::Point { x: 100, y: 100 }),
        Bounds::card(crate::layout::Point { x: 600, y: 400 }),
    ];
    let mut sides = vec![[Side::Top; 2]; 24];
    fit_sides(&graph, &bounds, &mut sides, &[true; 24], None);
    assert!(
        sides
            .iter()
            .flatten()
            .all(|s| matches!(s, Side::Left | Side::Right))
    );
    assert_eq!(required_heights(&graph, &sides, &[true; 24]), vec![425; 2]);
}

#[test]
fn shared_horizontal_bundle_terminals_do_not_overflow() {
    use crate::layout::Point;
    use crate::model::Edge;
    let resources: Vec<_> = (0..19)
        .map(|i| serde_json::json!({"address":format!("test.n{i:02}"),"type":"test"}))
        .collect();
    let raw = crate::plan::parse(
        &serde_json::json!({"format_version":"1.2","resource_changes":resources}).to_string(),
    )
    .unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    let bounds: Vec<_> = (0..19)
        .map(|i| {
            Bounds::card(Point {
                x: 100 + i * 600,
                y: 400,
            })
        })
        .collect();
    for source in [false, true] {
        graph.edges = (1..19)
            .map(|i| Edge::from(if source { (0, i) } else { (i, 0) }))
            .collect();
        for side in [Side::Top, Side::Bottom] {
            let paths: Vec<_> = graph
                .edges
                .iter()
                .map(|edge| {
                    [edge.from, edge.to]
                        .into_iter()
                        .map(|node| Point {
                            x: bounds[node].origin.x + bounds[node].width / 2,
                            y: bounds[node].origin.y
                                + if side == Side::Bottom {
                                    bounds[node].height
                                } else {
                                    0
                                },
                        })
                        .collect()
                })
                .collect();
            let mut sides = vec![[side; 2]; 18];
            assert!(!crowded(&graph, &paths, &sides));
            fit_sides(&graph, &bounds, &mut sides, &[true; 18], Some(&paths));
            assert_eq!(sides, vec![[side; 2]; 18]);
            fit_sides(&graph, &bounds, &mut sides, &[true; 18], None);
            assert!(
                sides
                    .iter()
                    .all(|ends| matches!(ends[usize::from(!source)], Side::Left | Side::Right))
            );
        }
    }
}

#[test]
fn incident_slots_follow_peer_geometry_in_both_directions_on_every_side() {
    use crate::layout::Point;
    use crate::model::architecture::Edge;
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.z","type":"test"},{"address":"test.a","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![
        Edge::from((0, 1)),
        Edge::from((0, 2)),
        Edge::from((1, 0)),
        Edge::from((2, 0)),
    ];
    let mut bounds = vec![
        Bounds {
            origin: Point { x: 300, y: 300 },
            width: 200,
            height: 200,
        },
        Bounds::card(Point { x: 100, y: 100 }),
        Bounds::card(Point { x: 600, y: 600 }),
    ];
    let incident = |graph: &Graph| {
        let mut result = vec![Vec::new(); graph.nodes.len()];
        for (i, e) in graph.edges.iter().enumerate() {
            result[e.from].push((i, true));
            result[e.to].push((i, false));
        }
        result
    };
    let (sources, targets) = assign(&graph, &bounds, &incident(&graph));
    for side in 0..4 {
        assert!(sources[0][side] < sources[1][side]);
        assert!(targets[2][side] < targets[3][side]);
    }
    graph.edges.reverse();
    let (s, t) = assign(&graph, &bounds, &incident(&graph));
    assert_eq!(sources, s.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(targets, t.into_iter().rev().collect::<Vec<_>>());
    bounds[2] = bounds[1];
    let tied = assign(&graph, &bounds, &incident(&graph));
    graph.edges.reverse();
    let again = assign(&graph, &bounds, &incident(&graph));
    assert_eq!(tied.0, again.0.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(tied.1, again.1.into_iter().rev().collect::<Vec<_>>());
}
