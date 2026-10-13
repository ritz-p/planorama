use super::*;
use crate::layout::ContainmentTree;
use crate::model::Edge;

#[test]
fn port_exchanges_reject_marker_corner_overlap_without_centerline_collisions() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.lower","type":"test"},{"address":"test.upper","type":"test"},{"address":"test.marker","type":"test"},{"address":"test.peer","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![Edge::from((0, 1)), Edge::from((0, 2)), Edge::from((3, 4))];
    let numbered = crate::semantic::transform(
        &crate::plan::parse(include_str!("../../../fixtures/association-plan.json")).unwrap(),
    )
    .0;
    let relationship = numbered
        .relationships
        .iter()
        .find(|r| r.kind == crate::model::EdgeKind::Association)
        .unwrap();
    for index in [0, 2] {
        let edge = &mut graph.edges[index];
        edge.kind = relationship.kind;
        let mut relationship = relationship.clone();
        relationship.from = graph.nodes[edge.from].entity.id.clone();
        relationship.to = graph.nodes[edge.to].entity.id.clone();
        graph.relationships.push(relationship);
    }
    let bounds: Vec<_> = [
        (0, 0, 100, 300),
        (600, 750, 100, 100),
        (800, 0, 100, 100),
        (164, 205, 80, 100),
        (300, 1050, 100, 100),
    ]
    .into_iter()
    .map(|(x, y, width, height)| Bounds {
        origin: Point { x, y },
        width,
        height,
    })
    .collect();
    let paths: Vec<Vec<_>> = [
        vec![(100, 100), (500, 100), (500, 800), (600, 800)],
        vec![(100, 200), (400, 200), (400, 50), (800, 50)],
        vec![(164, 217), (122, 217), (122, 1100), (300, 1100)],
    ]
    .into_iter()
    .map(|path| path.into_iter().map(|(x, y)| Point { x, y }).collect())
    .collect();
    let mut layout = Layout {
        scopes: vec![],
        header_heights: bounds.iter().map(|b| b.height).collect(),
        positions: bounds.iter().map(|b| b.origin).collect(),
        bounds,
        containers: vec![],
        containment: ContainmentTree::new(&graph),
        width: 1200,
        height: 1300,
        bands: vec![],
        paths: paths.clone(),
        junctions: vec![],
    };
    let mut plain = graph.clone();
    plain.relationships.remove(0);
    let mut exchanged = paths.clone();
    for (edge, other) in [(0, 1), (1, 0)] {
        exchanged[edge][0] = paths[other][0];
        exchanged[edge][1].y = paths[other][0].y;
    }
    assert_eq!(crossings(&exchanged[0], &exchanged[1]), 0);
    let terminals: Vec<_> = exchanged
        .iter()
        .map(|path| Some([terminal(path, 0), terminal(path, 1)]))
        .collect();
    let clearances = |graph: &Graph| {
        let numbers = relationship_markers::numbers(graph);
        terminals
            .iter()
            .enumerate()
            .map(|(index, pair)| {
                relationship_markers::terminals(
                    graph,
                    index,
                    numbers[index],
                    pair.unwrap().map(|p| p.side),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(!marker_collision(
        &exchanged,
        &terminals,
        &clearances(&plain)
    ));
    assert!(marker_collision(
        &exchanged,
        &terminals,
        &clearances(&graph)
    ));
    let numbers = relationship_markers::numbers(&graph);
    let reservations: Vec<_> = exchanged
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let pair = [terminal(path, 0), terminal(path, 1)];
            ports::terminal_corridors(
                pair[0],
                pair[1],
                relationship_markers::terminals(
                    &graph,
                    index,
                    numbers[index],
                    pair.map(|p| p.side),
                ),
            )
        })
        .collect();
    assert!(reservations.iter().enumerate().all(|(index, own)| {
        exchanged.iter().enumerate().all(|(other, path)| {
            index == other
                || path
                    .windows(2)
                    .all(|p| own.iter().all(|&b| !crosses(p[0], p[1], b)))
        })
    }));
    let a = relationship_markers::bounds(numbers[0][0].unwrap(), terminal(&exchanged[0], 0));
    let b = relationship_markers::bounds(numbers[2][0].unwrap(), terminal(&exchanged[2], 0));
    assert!(a.origin.x < b.right() && b.origin.x < a.right());
    assert!(a.origin.y < b.origin.y + b.height && b.origin.y < a.origin.y + a.height);
    refine(&graph, &mut layout);
    assert_eq!(layout.paths, paths);
}

#[test]
fn crossed_ports_are_exchanged_for_every_side_and_role() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.lower","type":"test"},{"address":"test.upper","type":"test"}]}"#).unwrap();
    for rotation in 0..4 {
        for roles in [[false, false], [true, true], [false, true]] {
            let mut graph = crate::semantic::transform(&raw).0;
            graph.edges = vec![Edge::from((0, 1)), Edge::from((0, 2))];
            for (edge, reverse) in graph.edges.iter_mut().zip(roles) {
                if reverse {
                    std::mem::swap(&mut edge.from, &mut edge.to);
                }
            }
            let transform = |mut p: Point| {
                for _ in 0..rotation {
                    p = Point {
                        x: 2000 - p.y,
                        y: p.x,
                    };
                }
                p
            };
            let bounds: Vec<_> = [
                (100, 300, 100, 200),
                (600, 700, 100, 100),
                (800, 100, 100, 1400),
            ]
            .into_iter()
            .map(|(x, y, w, h)| {
                let a = transform(Point { x, y });
                let b = transform(Point { x: x + w, y: y + h });
                Bounds {
                    origin: Point {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    width: a.x.abs_diff(b.x),
                    height: a.y.abs_diff(b.y),
                }
            })
            .collect();
            let paths: Vec<Vec<Point>> = [
                vec![(200, 350), (500, 350), (500, 750), (600, 750)],
                vec![(200, 450), (400, 450), (400, 150), (800, 150)],
            ]
            .into_iter()
            .zip(roles)
            .map(|(p, reverse)| {
                let mut p: Vec<_> = p
                    .into_iter()
                    .map(|(x, y)| transform(Point { x, y }))
                    .collect();
                if reverse {
                    p.reverse();
                }
                p
            })
            .collect();
            assert_eq!(crossings(&paths[0], &paths[1]), 1);
            let mut layout = Layout {
                scopes: vec![],
                header_heights: vec![200, 100, 100],
                bounds,
                containers: vec![],
                containment: ContainmentTree::new(&graph),
                width: 2000,
                height: 2000,
                positions: vec![],
                bands: vec![],
                paths,
                junctions: vec![],
            };
            let original = layout.paths.clone();
            let axis = |p: Point| if rotation % 2 == 0 { p.y } else { p.x };
            let centers: Vec<_> = layout
                .bounds
                .iter()
                .map(|b| {
                    axis(Point {
                        x: b.origin.x + b.width / 2,
                        y: b.origin.y + b.height / 2,
                    })
                })
                .collect();
            let hub_ports: Vec<_> = original
                .iter()
                .zip(roles)
                .map(|(p, reverse)| axis(terminal(p, usize::from(reverse)).point))
                .collect();
            assert_eq!(centers[1].cmp(&centers[2]), hub_ports[0].cmp(&hub_ports[1]));
            layout.junctions = vec![transform(Point { x: 400, y: 350 })];
            refine(&graph, &mut layout);
            assert_eq!(original, layout.paths);
            layout.junctions.clear();
            refine(&graph, &mut layout);
            assert_eq!(
                crossings(&layout.paths[0], &layout.paths[1]),
                0,
                "rotation={rotation}, roles={roles:?}"
            );
            assert!(
                layout.paths.iter().map(|p| length(p)).sum::<usize>()
                    <= original.iter().map(|p| length(p)).sum()
            );
            let expected = layout.paths.clone();
            graph.edges.reverse();
            layout.paths = original.into_iter().rev().collect();
            refine(&graph, &mut layout);
            assert_eq!(expected, layout.paths.into_iter().rev().collect::<Vec<_>>());
        }
    }
}
