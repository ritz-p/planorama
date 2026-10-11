use super::*;
use crate::layout::ContainmentTree;
use crate::model::Edge;

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
                crate::layout::relationship_markers::ends(&graph)[i].map(|n| if n {
                    crate::layout::relationship_markers::clearance(&graph)
                } else {
                    16
                })
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
fn soft_spacing_keeps_clean_routes_when_capacity_needs_excessive_growth() {
    let graph = corridor_graph(7);
    let mut layout = narrow(&graph, false);
    let before = expansion::Snapshot::capture(&layout);
    refine(&graph, &mut layout);
    assert_eq!((layout.width, layout.height), (before.width, before.height));
    assert!(measure(&graph, &layout.paths).close_length > 0);
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
        scope: crate::model::architecture::DeploymentScope {
            provider: "test".into(),
            region: None,
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
}
