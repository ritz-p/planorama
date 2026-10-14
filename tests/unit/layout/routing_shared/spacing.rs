use super::*;
use crate::layout::ContainmentTree;
use crate::model::Edge;

#[path = "spacing_calibration.rs"]
mod calibration;

fn graph() -> Graph {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.c","type":"test"},{"address":"test.d","type":"test"},{"address":"test.e","type":"test"},{"address":"test.f","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![Edge::from((0, 1)), Edge::from((2, 3))];
    let numbered = crate::semantic::transform(
        &crate::plan::parse(include_str!("../../../fixtures/association-plan.json")).unwrap(),
    )
    .0;
    let mut relationship = numbered
        .relationships
        .iter()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, crate::model::RelationshipProvenance::Resource { .. }))
        })
        .unwrap()
        .clone();
    graph.edges[0].kind = relationship.kind;
    relationship.from = graph.nodes[0].entity.id.clone();
    relationship.to = graph.nodes[1].entity.id.clone();
    graph.relationships = vec![relationship];
    graph
}

fn corridor_graph(count: usize) -> Graph {
    let resources: Vec<_> = (0..count * 2 + 2)
        .map(|i| serde_json::json!({"address":format!("test.n{i:02}"),"type":"test"}))
        .collect();
    let raw = crate::plan::parse(
        &serde_json::json!({"format_version":"1.2","resource_changes":resources}).to_string(),
    )
    .unwrap();
    let mut result = crate::semantic::transform(&raw).0;
    result.edges = (0..count).map(|i| Edge::from((i * 2, i * 2 + 1))).collect();
    let mut numbered = graph().relationships.remove(0);
    numbered.from = result.nodes[0].entity.id.clone();
    numbered.to = result.nodes[1].entity.id.clone();
    result.edges[0].kind = numbered.kind;
    result.relationships = vec![numbered];
    if count > 2 {
        result.edges[2].kind = crate::model::EdgeKind::Connection;
    }
    result
}

fn narrow(graph: &Graph, transpose: bool) -> Layout<'_> {
    let count = graph.edges.len();
    let transform = |p: Point| {
        if transpose {
            Point { x: p.y, y: p.x }
        } else {
            p
        }
    };
    let mut bounds = Vec::new();
    let mut paths = Vec::new();
    for i in 0..count {
        for (x, y) in [(100, 100 + i * 100), (800, 1200 + i * 100)] {
            bounds.push(Bounds {
                origin: Point { x, y },
                width: 100,
                height: 60,
            });
        }
        let x = 493 + i * 2;
        paths.push(vec![
            Point {
                x: 200,
                y: 130 + i * 100,
            },
            Point {
                x,
                y: 130 + i * 100,
            },
            Point {
                x,
                y: 1230 + i * 100,
            },
            Point {
                x: 800,
                y: 1230 + i * 100,
            },
        ]);
    }
    bounds.extend([
        Bounds {
            origin: Point { x: 100, y: 800 },
            width: 390,
            height: 300,
        },
        Bounds {
            origin: Point { x: 510, y: 800 },
            width: 390,
            height: 300,
        },
    ]);
    if transpose {
        for b in &mut bounds {
            b.origin = transform(b.origin);
            std::mem::swap(&mut b.width, &mut b.height);
        }
        for p in paths.iter_mut().flatten() {
            *p = transform(*p);
        }
    }
    Layout {
        scopes: vec![],
        header_heights: bounds.iter().map(|b| b.height).collect(),
        bounds,
        containers: vec![],
        containment: ContainmentTree::new(graph),
        width: if transpose { 2200 } else { 1100 },
        height: if transpose { 1100 } else { 2200 },
        positions: vec![],
        bands: vec![],
        paths,
        junctions: vec![],
    }
}

fn single_border_corridor(graph: &Graph, transpose: bool, run: usize) -> Layout<'_> {
    let mut layout = narrow(graph, false);
    layout.bounds[2].height = run;
    layout.bounds[3].origin.x = 502;
    layout.bounds[3].height = run;
    if transpose {
        for bounds in &mut layout.bounds {
            std::mem::swap(&mut bounds.origin.x, &mut bounds.origin.y);
            std::mem::swap(&mut bounds.width, &mut bounds.height);
        }
        for point in layout.paths.iter_mut().flatten() {
            std::mem::swap(&mut point.x, &mut point.y);
        }
        std::mem::swap(&mut layout.width, &mut layout.height);
    }
    layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    layout
}

#[test]
fn a_single_path_reserves_visible_border_clearance_in_both_axes() {
    for kind in [
        crate::model::EdgeKind::Dependency,
        crate::model::EdgeKind::Association,
        crate::model::EdgeKind::Connection,
    ] {
        let mut graph = corridor_graph(1);
        graph.edges[0].kind = kind;
        if kind != crate::model::EdgeKind::Association {
            graph.relationships.clear();
        }
        for transpose in [false, true] {
            for run in [32, 300] {
                let mut layout = single_border_corridor(&graph, transpose, run);
                let before = expansion::Snapshot::capture(&layout);
                let initial = border_metrics(&graph, &layout)[0];
                assert_eq!(measure(&graph, &layout.paths), Metrics::default());
                assert!(initial.minimum_deficit > 0);
                let corridors = plan::corridors(&graph, &layout);
                assert!(corridors.iter().any(|c| c.border && c.lanes.len() == 1));
                refine(&graph, &mut layout);
                let after = border_metrics(&graph, &layout)[0];
                assert_eq!(after.minimum_deficit, 0, "{kind:?}, {transpose}, {run}");
                assert!(after.preferred_deficit < initial.preferred_deficit);
                let straight_gap = 10 * 4
                    - (crate::view::geometry::relationship_stroke(kind)
                        + crate::view::geometry::CARD_STROKE)
                        / 2;
                assert!(straight_gap >= super::super::borders::PREFERRED * 4);
                assert!(after.cost() < initial.cost());
                assert_eq!(
                    (layout.width - before.width, layout.height - before.height),
                    if transpose { (0, 8) } else { (8, 0) },
                );
                let gap = if transpose {
                    layout.bounds[3].origin.y - layout.bounds[2].origin.y - layout.bounds[2].height
                } else {
                    layout.bounds[3].origin.x - layout.bounds[2].right()
                };
                assert_eq!(gap, 20);
                let path = &layout.paths[0];
                assert_eq!(Shape::measure(path).bends, 2);
                assert!(Shape::measure(path).fits(Shape::measure(&before.paths[0])));
                assert!(ports::valid_with_clearances(
                    path,
                    terminal(path, 0),
                    terminal(path, 1),
                    &barriers(&graph, &layout, 0),
                    crate::layout::relationship_markers::terminals(
                        &graph,
                        0,
                        crate::layout::relationship_markers::numbers(&graph)[0],
                        [terminal(path, 0).side, terminal(path, 1).side],
                    ),
                ));
                let expected = (layout.bounds.clone(), layout.paths.clone());
                let mut repeat = single_border_corridor(&graph, transpose, run);
                refine(&graph, &mut repeat);
                assert_eq!(expected, (repeat.bounds, repeat.paths));
            }
        }
    }
}

#[test]
fn protected_border_paths_keep_geometry_and_the_residual_measurable() {
    let graph = corridor_graph(1);
    let mut layout = single_border_corridor(&graph, false, 300);
    layout.junctions = vec![Point { x: 493, y: 600 }];
    layout.scopes.push(crate::layout::ScopePanel {
        account_panel: false,
        scope: crate::model::architecture::DeploymentScope {
            provider: "test".into(),
            region: None,
            account: None,
            global: true,
        },
        bounds: Bounds {
            origin: Point { x: 0, y: 0 },
            width: layout.width,
            height: layout.height,
        },
    });
    layout.bands.push(crate::layout::Band {
        label: "test",
        top: 40,
        height: layout.height - 40,
    });
    let before = expansion::Snapshot::capture(&layout);
    let initial = border_metrics(&graph, &layout);
    assert!(initial[0].minimum_deficit > 0);
    assert!(
        plan::corridors(&graph, &layout)
            .iter()
            .any(|c| c.demand(&layout).is_some())
    );
    refine(&graph, &mut layout);
    assert_eq!(layout.bounds, before.bounds);
    assert_eq!(layout.paths, before.paths);
    assert_eq!(layout.junctions, before.junctions);
    assert_eq!((layout.width, layout.height), (before.width, before.height));
    assert_eq!(layout.scopes[0].bounds.width, before.width);
    assert_eq!(
        (layout.bands[0].top, layout.bands[0].height),
        (40, before.height - 40)
    );
    assert_eq!(
        layout.positions,
        layout.bounds.iter().map(|b| b.origin).collect::<Vec<_>>()
    );
    assert_eq!(border_metrics(&graph, &layout), initial);
}

#[test]
fn later_rerouting_cannot_transfer_overlap_onto_an_earlier_route() {
    let mut graph = corridor_graph(3);
    graph.relationships.clear();
    graph.edges[0].kind = crate::model::EdgeKind::Dependency;
    let mut layout = narrow(&graph, false);
    for (i, y) in [100, 800, 300, 700, 200, 750].into_iter().enumerate() {
        layout.bounds[i].origin.y = y - 20;
        layout.bounds[i].height = 40;
    }
    layout.bounds[6] = Bounds {
        origin: Point { x: 501, y: 350 },
        width: 30,
        height: 200,
    };
    layout.bounds[7] = Bounds {
        origin: Point { x: 950, y: 1000 },
        width: 20,
        height: 20,
    };
    let point = |x, y| Point { x, y };
    layout.paths = vec![
        vec![
            point(200, 100),
            point(500, 100),
            point(500, 800),
            point(800, 800),
        ],
        vec![
            point(200, 300),
            point(700, 300),
            point(700, 600),
            point(600, 600),
            point(600, 700),
            point(800, 700),
        ],
        vec![
            point(200, 200),
            point(700, 200),
            point(700, 600),
            point(600, 600),
            point(600, 750),
            point(800, 750),
        ],
    ];
    let before = expansion::Snapshot::capture(&layout);
    let baseline: Vec<_> = before.paths.iter().map(|p| Shape::measure(p)).collect();
    let planned = plan::Corridor {
        border: false,
        minimum_margin: PREFERRED,
        preferred_margin: PREFERRED,
        urgent: false,
        deficit_cost: 0,
        hidden: std::collections::BTreeSet::new(),
        horizontal: false,
        low: 100,
        high: 800,
        lanes: vec![
            plan::Lane {
                edge: 0,
                original: 500,
                coordinate: 500,
            },
            plan::Lane {
                edge: 1,
                original: 700,
                coordinate: 516,
            },
        ],
    };
    let overlap = |paths: &[Vec<Point>], i: usize| {
        paths
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| super::super::refinement::overlap(&paths[i], p))
            .sum::<usize>()
    };
    assert_eq!(overlap(&before.paths, 0), 0);
    assert_eq!(overlap(&before.paths, 1), 500);
    let mut transferred = before.paths.clone();
    transferred[1] = vec![
        point(200, 300),
        point(500, 300),
        point(500, 700),
        point(800, 700),
    ];
    assert_eq!(Shape::measure(&transferred[1]).bends, 2);
    assert_eq!(overlap(&transferred, 0), 400);
    assert_eq!(overlap(&transferred, 1), 400);
    assert!(!reroute::preserves_overlap(
        &before.paths,
        &transferred,
        &[0, 1]
    ));
    let accepted = reroute::apply(&graph, &mut layout, &planned, &before, &baseline, false);
    if accepted {
        for (i, &shape) in baseline.iter().enumerate().take(2) {
            assert!(overlap(&layout.paths, i) <= overlap(&before.paths, i));
            assert!(Shape::measure(&layout.paths[i]).fits(shape));
        }
    }
    let expected = before.paths.clone();
    before.restore(&mut layout);
    assert_eq!(layout.paths, expected);
}

#[test]
fn expansion_cannot_hide_increased_overlap_with_a_protected_path() {
    let mut graph = corridor_graph(2);
    graph.relationships.clear();
    graph.edges[0].kind = crate::model::EdgeKind::Dependency;
    let mut layout = narrow(&graph, false);
    for (i, y) in [370, 370, 170, 570].into_iter().enumerate() {
        layout.bounds[i].origin.y = y;
    }
    layout.paths = vec![
        vec![Point { x: 200, y: 400 }, Point { x: 800, y: 400 }],
        vec![
            Point { x: 200, y: 200 },
            Point { x: 300, y: 200 },
            Point { x: 300, y: 400 },
            Point { x: 600, y: 400 },
            Point { x: 600, y: 600 },
            Point { x: 800, y: 600 },
        ],
    ];
    layout.junctions = vec![Point { x: 600, y: 500 }];
    let before = expansion::Snapshot::capture(&layout);
    let baseline: Vec<_> = before.paths.iter().map(|p| Shape::measure(p)).collect();
    let planned = plan::Corridor {
        border: false,
        minimum_margin: PREFERRED,
        preferred_margin: PREFERRED,
        urgent: false,
        deficit_cost: 0,
        hidden: std::collections::BTreeSet::new(),
        horizontal: false,
        low: 200,
        high: 600,
        lanes: vec![plan::Lane {
            edge: 0,
            original: 500,
            coordinate: 516,
        }],
    };
    assert_eq!(
        super::super::refinement::overlap(&before.paths[0], &before.paths[1]),
        300
    );
    assert!(expansion::allocate(&mut layout, false, 500, 44));
    assert_eq!(
        super::super::refinement::overlap(&layout.paths[0], &layout.paths[1]),
        344
    );
    assert!(!reroute::apply(
        &graph,
        &mut layout,
        &planned,
        &before,
        &baseline,
        true
    ));
    let expected = (
        before.bounds.clone(),
        before.paths.clone(),
        before.junctions.clone(),
    );
    before.restore(&mut layout);
    assert_eq!((layout.bounds, layout.paths, layout.junctions), expected);
}

#[test]
fn corridor_capacity_expands_then_reroutes_mixed_edges_into_whole_lanes() {
    for transpose in [false, true] {
        let mut graph = corridor_graph(3);
        let mut layout = narrow(&graph, transpose);
        let initial = expansion::Snapshot::capture(&layout);
        let old_shape: Vec<_> = layout.paths.iter().map(|p| Shape::measure(p)).collect();
        let first = plan::corridors(&graph, &layout).remove(0);
        assert_eq!(first.lanes.len(), 3);
        assert_eq!(first.demand(&layout), Some((500, 44)));
        refine(&graph, &mut layout);
        assert_eq!(measure(&graph, &layout.paths).close_length, 0);
        assert_eq!(
            (layout.width, layout.height),
            if transpose {
                (2200, 1144)
            } else {
                (1144, 2200)
            }
        );
        for (i, path) in layout.paths.iter().enumerate() {
            assert_eq!(Shape::measure(path).bends, 2);
            assert_eq!(Shape::measure(path).offsets, 0);
            assert!(Shape::measure(path).fits(old_shape[i]));
            assert_ne!(path[1], initial.paths[i][1]);
            assert!(ports::valid_with_clearances(
                path,
                terminal(path, 0),
                terminal(path, 1),
                &barriers(&graph, &layout, i),
                crate::layout::relationship_markers::terminals(
                    &graph,
                    i,
                    crate::layout::relationship_markers::numbers(&graph)[i],
                    [terminal(path, 0).side, terminal(path, 1).side],
                )
            ));
        }
        let expected = (layout.bounds.clone(), layout.paths.clone());
        drop(layout);
        graph.edges.reverse();
        let mut layout = narrow(&graph, transpose);
        layout.paths = initial.paths.into_iter().rev().collect();
        refine(&graph, &mut layout);
        assert_eq!(
            expected,
            (layout.bounds, layout.paths.into_iter().rev().collect())
        );
    }
}

#[test]
fn corridor_candidates_follow_actual_peers_when_a_protected_lane_stays_in_place() {
    let mut graph = corridor_graph(2);
    graph.relationships.clear();
    graph.edges[0].kind = crate::model::EdgeKind::Dependency;
    let point = |x, y| Point { x, y };
    for transpose in [false, true] {
        let mut layout = narrow(&graph, false);
        layout.bounds[2].origin.y = 0;
        layout.bounds[3].origin.y = 1400;
        layout.bounds[4].width = 300;
        layout.bounds[5].origin.x = 600;
        layout.bounds[5].width = 300;
        layout.paths = vec![
            vec![
                point(200, 130),
                point(500, 130),
                point(500, 1230),
                point(800, 1230),
            ],
            vec![
                point(200, 30),
                point(502, 30),
                point(502, 1430),
                point(800, 1430),
            ],
        ];
        layout.junctions = vec![point(502, 700)];
        if transpose {
            for bounds in &mut layout.bounds {
                std::mem::swap(&mut bounds.origin.x, &mut bounds.origin.y);
                std::mem::swap(&mut bounds.width, &mut bounds.height);
            }
            for p in layout
                .paths
                .iter_mut()
                .flatten()
                .chain(&mut layout.junctions)
            {
                std::mem::swap(&mut p.x, &mut p.y);
            }
            std::mem::swap(&mut layout.width, &mut layout.height);
        }
        layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
        let before = expansion::Snapshot::capture(&layout);
        let before_borders = border_metrics(&graph, &layout);
        let baseline: Vec<_> = before.paths.iter().map(|p| Shape::measure(p)).collect();
        let planned = plan::Corridor {
            border: false,
            minimum_margin: PREFERRED,
            preferred_margin: PREFERRED,
            urgent: false,
            deficit_cost: 0,
            hidden: std::collections::BTreeSet::new(),
            horizontal: transpose,
            low: 130,
            high: 1230,
            lanes: vec![
                plan::Lane {
                    edge: 0,
                    original: 500,
                    coordinate: 500,
                },
                plan::Lane {
                    edge: 1,
                    original: 502,
                    coordinate: 516,
                },
            ],
        };
        assert_eq!(measure(&graph, &layout.paths).deficit, 1100 * 14);
        assert!(reroute::apply(
            &graph,
            &mut layout,
            &planned,
            &before,
            &baseline,
            false
        ));
        assert_eq!(measure(&graph, &layout.paths).close_length, 0);
        assert_eq!(layout.paths[1], before.paths[1]);
        assert_eq!(layout.junctions, before.junctions);
        assert_eq!(layout.bounds, before.bounds);
        let axis = |point: Point| if transpose { point.y } else { point.x };
        assert_eq!(axis(layout.paths[0][1]), 486);
        assert_eq!(axis(layout.paths[0][2]), 486);
        assert_eq!(Shape::measure(&layout.paths[0]), baseline[0]);
        assert!(quality::improves(
            &graph,
            &before.paths,
            &layout.paths,
            &before_borders,
            &border_metrics(&graph, &layout),
            false,
        ));
    }
}

#[test]
fn bounded_peer_axes_keep_clear_alternatives_across_the_endpoint_region() {
    let point = |x, y| Point { x, y };
    let corridor = plan::Corridor {
        border: false,
        minimum_margin: PREFERRED,
        preferred_margin: PREFERRED,
        urgent: false,
        deficit_cost: 0,
        hidden: std::collections::BTreeSet::new(),
        horizontal: false,
        low: 400,
        high: 1800,
        lanes: vec![plan::Lane {
            edge: 0,
            original: 1234,
            coordinate: 1234,
        }],
    };
    for peer in [1632, 1650] {
        let mut paths = vec![vec![
            point(861, 400),
            point(1234, 400),
            point(1234, 1800),
            point(2419, 1800),
        ]];
        paths.extend(
            [1214, 1220, 1226, 1234, 1238, 1242, 1246, peer]
                .into_iter()
                .map(|x| vec![point(x, 400), point(x, 1800)]),
        );
        let axes = |paths: &[Vec<Point>]| {
            parallel::Index::new(paths, 0).axes(
                &corridor,
                [point(861, 400), point(2419, 1800)],
                [1234, peer + 8],
                1200..1264,
                4196,
            )
        };
        let expected = axes(&paths);
        assert_eq!(&expected[..2], &[1234, peer + 8]);
        assert_eq!(expected.len(), 32);
        assert!(expected.contains(&(peer + PREFERRED)));
        paths[1..].reverse();
        assert_eq!(axes(&paths), expected);
        paths.extend(
            (0..256).map(|i| vec![point(10000 + i * 16, 400), point(10000 + i * 16, 1800)]),
        );
        assert_eq!(axes(&paths), expected);
    }
}

#[test]
fn numbered_turns_keep_distinct_perpendicular_runs_when_spacing_a_corridor() {
    let mut graph = corridor_graph(2);
    let mut relationship = graph.relationships[0].clone();
    relationship.from = graph.nodes[2].entity.id.clone();
    relationship.to = graph.nodes[3].entity.id.clone();
    graph.edges[1].kind = relationship.kind;
    graph.relationships.push(relationship);
    let point = |x, y| Point { x, y };
    for transpose in [false, true] {
        let mut layout = narrow(&graph, false);
        for (index, (x, y)) in [
            (100, 100),
            (800, 450),
            (300, 100),
            (800, 750),
            (50, 1000),
            (50, 1200),
        ]
        .into_iter()
        .enumerate()
        {
            layout.bounds[index] = Bounds {
                origin: point(x, y),
                width: 100,
                height: 100,
            };
        }
        layout.bounds[4] = Bounds {
            origin: point(100, 350),
            width: 400,
            height: 50,
        };
        layout.paths = vec![
            vec![
                point(150, 200),
                point(150, 260),
                point(700, 260),
                point(700, 500),
                point(800, 500),
            ],
            vec![
                point(350, 200),
                point(350, 262),
                point(720, 262),
                point(720, 800),
                point(800, 800),
            ],
        ];
        if transpose {
            for bounds in &mut layout.bounds {
                std::mem::swap(&mut bounds.origin.x, &mut bounds.origin.y);
                std::mem::swap(&mut bounds.width, &mut bounds.height);
            }
            for p in layout.paths.iter_mut().flatten() {
                std::mem::swap(&mut p.x, &mut p.y);
            }
            std::mem::swap(&mut layout.width, &mut layout.height);
        }
        let before = expansion::Snapshot::capture(&layout);
        assert!(measure(&graph, &layout.paths).close_length >= 300);
        refine(&graph, &mut layout);
        assert_eq!(measure(&graph, &layout.paths).close_length, 0);
        assert_eq!(layout.bounds, before.bounds);
        assert_eq!(
            super::super::refinement::overlap(&layout.paths[0], &layout.paths[1]),
            0
        );
        let numbers = crate::layout::relationship_markers::numbers(&graph);
        for (i, path) in layout.paths.iter().enumerate() {
            assert_eq!(path.first(), before.paths[i].first());
            assert_eq!(path.last(), before.paths[i].last());
            assert_eq!(Shape::measure(path).bends, 3);
            assert_eq!(Shape::measure(path).reversals, 0);
            assert!(ports::valid_with_clearances(
                path,
                terminal(path, 0),
                terminal(path, 1),
                &barriers(&graph, &layout, i),
                crate::layout::relationship_markers::terminals(
                    &graph,
                    i,
                    numbers[i],
                    [terminal(path, 0).side, terminal(path, 1).side],
                ),
            ));
        }
    }
}

#[test]
fn seven_lanes_use_more_than_the_previous_axis_growth_budget() {
    let graph = corridor_graph(7);
    let mut layout = narrow(&graph, false);
    let before = expansion::Snapshot::capture(&layout);
    refine(&graph, &mut layout);
    assert!(layout.width > before.width + 64);
    assert_eq!(measure(&graph, &layout.paths).close_length, 0);
    assert!(
        layout
            .paths
            .iter()
            .all(|p| Shape::measure(p).bends == 2 && Shape::measure(p).offsets == 0)
    );
}

#[test]
fn whole_path_shape_rejects_offset_reentry_but_tolerates_short_end_stubs() {
    let clean = vec![Point { x: 100, y: 100 }, Point { x: 100, y: 700 }];
    let jagged = vec![
        Point { x: 100, y: 100 },
        Point { x: 100, y: 300 },
        Point { x: 116, y: 300 },
        Point { x: 116, y: 500 },
        Point { x: 100, y: 500 },
        Point { x: 100, y: 700 },
    ];
    let shape = Shape::measure(&jagged);
    assert_eq!(
        (
            shape.offsets,
            shape.s_jogs,
            shape.reversals,
            shape.backtracking
        ),
        (2, 1, 1, 16)
    );
    assert!(!shape.fits(Shape::measure(&clean)));
    let terminal = vec![
        Point { x: 100, y: 100 },
        Point { x: 116, y: 100 },
        Point { x: 116, y: 700 },
        Point { x: 132, y: 700 },
    ];
    assert_eq!(Shape::measure(&terminal).offsets, 0);
}

#[test]
fn failed_reroute_rolls_back_geometry_scopes_bands_and_protected_junctions() {
    let mut graph = corridor_graph(3);
    for edge in &mut graph.edges {
        edge.from = 0;
    }
    let mut layout = narrow(&graph, false);
    for path in &mut layout.paths {
        path[0].y = 130;
        path[1].y = 130;
    }
    layout.junctions = vec![Point { x: 400, y: 130 }];
    layout.scopes.push(crate::layout::ScopePanel {
        account_panel: false,
        scope: crate::model::architecture::DeploymentScope {
            provider: "test".into(),
            region: None,
            account: None,
            global: true,
        },
        bounds: Bounds {
            origin: Point { x: 0, y: 0 },
            width: 1100,
            height: 2200,
        },
    });
    layout.bands.push(crate::layout::Band {
        label: "test",
        top: 40,
        height: 2160,
    });
    let before = expansion::Snapshot::capture(&layout);
    let mut planned = plan::corridors(&graph, &layout).remove(0);
    let (cut, amount) = planned.demand(&layout).unwrap();
    assert!(expansion::allocate(&mut layout, false, cut, amount));
    planned.translate(cut, amount);
    planned.allocate(&layout);
    let baseline: Vec<_> = before.paths.iter().map(|p| Shape::measure(p)).collect();
    assert!(!reroute::apply(
        &graph,
        &mut layout,
        &planned,
        &before,
        &baseline,
        true
    ));
    before.restore(&mut layout);
    let before = expansion::Snapshot::capture(&layout);
    refine(&graph, &mut layout);
    assert_eq!((layout.width, layout.height), (1100, 2200));
    assert_eq!(layout.bounds, before.bounds);
    assert_eq!(layout.paths, before.paths);
    assert_eq!(layout.junctions, before.junctions);
    assert_eq!(layout.scopes[0].bounds.width, 1100);
    assert_eq!((layout.bands[0].top, layout.bands[0].height), (40, 2160));
}
#[test]
fn proximity_ignores_perpendicular_disjoint_and_short_endpoint_runs() {
    let graph = graph();
    let point = |x, y| Point { x, y };
    for paths in [
        vec![
            vec![point(100, 100), point(500, 100)],
            vec![point(300, 0), point(300, 300)],
        ],
        vec![
            vec![point(100, 100), point(500, 100)],
            vec![point(501, 105), point(900, 105)],
        ],
        vec![
            vec![point(100, 100), point(190, 100)],
            vec![point(100, 105), point(190, 105)],
        ],
    ] {
        assert_eq!(measure(&graph, &paths), Metrics::default());
        assert_eq!(
            parallel::Index::new(&paths, 0).deficit(&graph, &paths[0]),
            0
        );
    }
}

#[test]
fn shared_trunk_is_exempt_but_close_independent_branches_are_counted() {
    let mut graph = graph();
    graph.edges[1].from = 0;
    let paths = vec![
        vec![
            Point { x: 100, y: 100 },
            Point { x: 400, y: 100 },
            Point { x: 400, y: 300 },
            Point { x: 800, y: 300 },
        ],
        vec![
            Point { x: 100, y: 100 },
            Point { x: 400, y: 100 },
            Point { x: 400, y: 305 },
            Point { x: 800, y: 305 },
        ],
    ];
    let result = measure(&graph, &paths);
    assert_eq!(result.minimum, Some(5));
    assert_eq!(result.close_length, 368);
    assert_eq!(
        parallel::Index::new(&paths, 0).deficit(&graph, &paths[0]),
        result.deficit
    );
}

#[test]
fn minimum_border_deficit_gets_one_attempt_without_reordering_the_cost_prefix() {
    let original: Vec<_> = (0..9)
        .map(|edge| plan::Corridor {
            horizontal: false,
            low: 100,
            high: 500,
            lanes: vec![plan::Lane {
                edge,
                original: 400,
                coordinate: 400,
            }],
            border: edge == 8,
            minimum_margin: 6,
            preferred_margin: 10,
            urgent: edge == 8,
            deficit_cost: (900 - edge * 100) as u128,
            hidden: std::collections::BTreeSet::new(),
        })
        .collect();
    let order = |corridors: &[plan::Corridor]| {
        corridors
            .iter()
            .map(|c| c.lanes[0].edge)
            .collect::<Vec<_>>()
    };
    for remaining in [1, 3, 6] {
        let mut corridors = original.clone();
        plan::reserve_urgent(&mut corridors, remaining);
        assert_eq!(
            order(&corridors[..remaining - 1]),
            (0..remaining - 1).collect::<Vec<_>>()
        );
        assert_eq!(corridors[remaining - 1].lanes[0].edge, 8);
        assert_eq!(corridors.len(), original.len());
        let scheduled = order(&corridors);
        plan::reserve_urgent(&mut corridors, remaining);
        assert_eq!(order(&corridors), scheduled);
    }
    let mut corridors = original.clone();
    plan::reserve_urgent(&mut corridors, 0);
    assert_eq!(order(&corridors), order(&original));
    corridors[8].urgent = false;
    plan::reserve_urgent(&mut corridors, ATTEMPTS);
    assert_eq!(order(&corridors), order(&original));
}

#[test]
fn separate_runs_on_one_track_keep_their_urgency_and_attempt_identity() {
    let graph = corridor_graph(1);
    let mut layout = narrow(&graph, false);
    layout.bounds[2] = Bounds {
        origin: Point { x: 200, y: 207 },
        width: 100,
        height: 80,
    };
    layout.bounds[3] = Bounds {
        origin: Point { x: 500, y: 202 },
        width: 100,
        height: 80,
    };
    layout.paths[0] = vec![
        Point { x: 150, y: 200 },
        Point { x: 350, y: 200 },
        Point { x: 350, y: 150 },
        Point { x: 450, y: 150 },
        Point { x: 450, y: 200 },
        Point { x: 650, y: 200 },
    ];
    let corridors = plan::corridors(&graph, &layout);
    let first = corridors
        .iter()
        .find(|c| c.horizontal && c.low < 300 && c.high > 200)
        .unwrap();
    let second = corridors
        .iter()
        .find(|c| c.horizontal && c.low < 600 && c.high > 500)
        .unwrap();
    assert!(!first.urgent);
    assert!(second.urgent);
    assert_ne!(first.key(), second.key());
    let mut reserved = vec![first.clone(), second.clone()];
    plan::reserve_urgent(&mut reserved, 1);
    assert_eq!(reserved[0].key(), second.key());
}
