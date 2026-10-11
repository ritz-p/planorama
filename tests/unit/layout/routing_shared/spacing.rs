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

fn paths() -> Vec<Vec<Point>> {
    [
        vec![
            (200, 150),
            (300, 150),
            (300, 350),
            (700, 350),
            (700, 150),
            (800, 150),
        ],
        vec![
            (200, 650),
            (400, 650),
            (400, 355),
            (600, 355),
            (600, 650),
            (800, 650),
        ],
    ]
    .into_iter()
    .map(|p| p.into_iter().map(|(x, y)| Point { x, y }).collect())
    .collect()
}

#[test]
fn parallel_runs_separate_locally_or_expand_only_the_congested_gap() {
    for transpose in [false, true] {
        for congested in [false, true] {
            let mut graph = graph();
            let transform = |p: Point| {
                if transpose {
                    Point { x: p.y, y: p.x }
                } else {
                    p
                }
            };
            let mut bounds: Vec<_> = [
                (100, 100, 100, 100),
                (800, 100, 100, 100),
                (100, 600, 100, 100),
                (800, 600, 100, 100),
                (450, 200, 100, 140),
                (450, 360, 100, 140),
            ]
            .into_iter()
            .map(|(x, y, w, h)| Bounds {
                origin: transform(Point { x, y }),
                width: if transpose { h } else { w },
                height: if transpose { w } else { h },
            })
            .collect();
            if !congested {
                bounds[4].origin = Point { x: 1200, y: 1200 };
                bounds[5].origin = Point { x: 1400, y: 1400 };
            }
            let paths: Vec<_> = paths()
                .into_iter()
                .map(|p| p.into_iter().map(transform).collect())
                .collect();
            let mut layout = Layout {
                scopes: vec![],
                header_heights: bounds.iter().map(|b| b.height).collect(),
                bounds,
                containers: vec![],
                containment: ContainmentTree::new(&graph),
                width: 1600,
                height: 1600,
                positions: vec![],
                bands: vec![],
                paths,
                junctions: vec![],
            };
            let initial_paths = layout.paths.clone();
            let initial_bounds = layout.bounds.clone();
            let before = measure(&graph, &layout.paths);
            assert_eq!(before.minimum, Some(5));
            let baseline: Vec<_> = layout.paths.iter().map(|p| length(p)).collect();
            assert!(!expand(&graph, &mut layout, !transpose, 150, 11, &baseline));
            assert!(!expand(&graph, &mut layout, !transpose, 200, 11, &baseline));
            refine(&graph, &mut layout);
            assert_eq!(
                measure(&graph, &layout.paths).close_length,
                0,
                "transpose={transpose},congested={congested}"
            );
            assert_eq!(
                (layout.width, layout.height),
                if congested {
                    if transpose {
                        (1611, 1600)
                    } else {
                        (1600, 1611)
                    }
                } else {
                    (1600, 1600)
                }
            );
            for (i, p) in layout.paths.iter().enumerate() {
                assert!(length(p) <= length(&initial_paths[i]) + 64);
                assert!(p.len() <= initial_paths[i].len());
                let edge = &graph.edges[i];
                for (end, node) in [edge.from, edge.to].into_iter().enumerate() {
                    let a = terminal(&initial_paths[i], end).point;
                    let b = terminal(p, end).point;
                    assert_eq!(
                        (
                            a.x - initial_bounds[node].origin.x,
                            a.y - initial_bounds[node].origin.y
                        ),
                        (
                            b.x - layout.bounds[node].origin.x,
                            b.y - layout.bounds[node].origin.y
                        )
                    );
                }
                assert!(ports::valid_with_clearances(
                    p,
                    terminal(p, 0),
                    terminal(p, 1),
                    &barriers(&graph, &layout, i),
                    crate::layout::relationship_markers::ends(&graph)[i].map(|n| if n {
                        crate::layout::relationship_markers::clearance(&graph)
                    } else {
                        16
                    })
                ));
            }
            let expected = (layout.bounds.clone(), layout.paths.clone());
            graph.edges.reverse();
            layout.bounds = initial_bounds;
            layout.paths = initial_paths.into_iter().rev().collect();
            layout.width = 1600;
            layout.height = 1600;
            refine(&graph, &mut layout);
            assert_eq!(
                expected,
                (layout.bounds, layout.paths.into_iter().rev().collect())
            );
        }
    }
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
