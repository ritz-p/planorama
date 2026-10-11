use super::*;
use crate::layout::ContainmentTree;
use crate::model::Edge;

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
