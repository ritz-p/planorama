use super::*;

#[test]
fn endpoint_policy_tracks_kind_direction_color_and_final_ports() {
    let raw = crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
    for kind in [
        EdgeKind::Dependency,
        EdgeKind::Association,
        EdgeKind::Connection,
        EdgeKind::Containment,
    ] {
        for directionality in [Directionality::Directed, Directionality::Undirected] {
            let mut graph = crate::semantic::transform(&raw).0;
            graph.edges[0].kind = kind;
            graph.edges[0].change.as_mut().unwrap().directionality = directionality;
            let mut layout = Layout::new(&graph);
            layout.paths[0] = vec![Point { x: 100, y: 200 }, Point { x: 300, y: 200 }];
            let output = crate::svg::render(&graph, &layout);
            let doc = roxmltree::Document::parse(&output).unwrap();
            if kind == EdgeKind::Connection {
                let path = doc
                    .descendants()
                    .find(|n| {
                        n.has_tag_name("path")
                            && n.attribute("data-edge-kind") == Some("connection")
                    })
                    .unwrap();
                assert_eq!(path.attribute("data-directionality"), Some("undirected"));
                assert_eq!(path.attribute("marker-end"), None);
                assert!(
                    path.descendants()
                        .any(|n| n.has_tag_name("title")
                            && n.text().is_some_and(|t| t.contains('↔')))
                );
            }
            let circles: Vec<_> = doc
                .descendants()
                .filter(|n| n.attribute("data-endpoint").is_some())
                .collect();
            let expected = match kind {
                EdgeKind::Containment => 0,
                EdgeKind::Dependency => 1,
                _ => 2,
            };
            assert_eq!(circles.len(), expected);
            for (i, circle) in circles.iter().enumerate() {
                assert_eq!(
                    circle.attribute("cx"),
                    Some(if i == 0 { "100" } else { "300" })
                );
                assert_eq!(circle.attribute("cy"), Some("200"));
                assert_eq!(
                    circle.attribute("stroke"),
                    Some(crate::svg::relationships::stroke(&graph.edges[0]))
                );
            }
            let arrow = doc
                .descendants()
                .find(|n| n.attribute("data-endpoint-arrow").is_some());
            assert_eq!(
                arrow.is_some(),
                expected == 2 && graph.edges[0].directionality() == Directionality::Directed
            );
            if let Some(arrow) = arrow {
                assert!(arrow.range().start > circles[1].range().start);
                assert_eq!(
                    arrow.attribute("d"),
                    Some("M 293.7 196.5 L 300.7 200 L 293.7 203.5 Z")
                );
                assert!(output.contains("marker-end=\"url(#arrow-create)\""));
            }
            layout.paths[0].clear();
            assert!(render(&graph, &layout).is_empty());
        }
    }
}

#[test]
fn parallel_numbered_and_bundled_paths_keep_each_endpoint_and_junction() {
    for fixture in [
        include_str!("../../fixtures/aws-relationships-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
        include_str!("../../fixtures/spanning-dense-plan.json"),
    ] {
        let graph = crate::semantic::transform(&crate::plan::parse(fixture).unwrap()).0;
        let layout = Layout::new(&graph);
        let output = crate::svg::render(&graph, &layout);
        let doc = roxmltree::Document::parse(&output).unwrap();
        let groups: Vec<_> = doc
            .descendants()
            .filter(|n| n.attribute("data-endpoints").is_some())
            .collect();
        let edges = graph
            .edges
            .iter()
            .zip(&layout.paths)
            .filter(|(edge, path)| edge.kind != EdgeKind::Containment && path.len() >= 2);
        for (group, (edge, path)) in groups.iter().zip(edges) {
            let circles: Vec<_> = group
                .children()
                .filter(|n| n.has_tag_name("circle"))
                .collect();
            let points: Vec<_> = if edge.kind == EdgeKind::Dependency {
                vec![path.first().unwrap()]
            } else {
                vec![path.first().unwrap(), path.last().unwrap()]
            };
            assert_eq!(circles.len(), points.len());
            for (circle, point) in circles.iter().zip(points) {
                assert_eq!(
                    circle.attribute("cx").unwrap().parse::<usize>().unwrap(),
                    point.x
                );
                assert_eq!(
                    circle.attribute("cy").unwrap().parse::<usize>().unwrap(),
                    point.y
                );
            }
        }
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_tag_name("circle") && n.attribute("data-endpoint").is_none())
                .count(),
            layout.junctions.len()
        );
        let last_card = doc
            .descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .map(|n| n.range().end)
            .max()
            .unwrap();
        assert!(groups.iter().all(|g| g.range().start > last_card));
        if let Some(badge) = doc
            .descendants()
            .find(|n| n.attribute("data-relationship-marker").is_some())
        {
            assert!(groups.iter().all(|g| g.range().end < badge.range().start));
        }
    }
}
