use super::*;

#[test]
fn namespacing_only_rewrites_fragment_reference_attributes() {
    let input = r##"<svg><g id="paint" data-action-reason="url(#ticket)"><title>url(#ticket)</title></g><path fill="url(#paint)" stroke="url(#paint)" clip-path="url(#paint)" mask="url(#paint)" marker-end="url(#paint)"/><use href="#paint"/></svg>"##;
    let scoped = namespace(input, "state-x-");
    assert!(scoped.contains("data-action-reason=\"url(#ticket)\""));
    assert!(scoped.contains("<title>url(#ticket)</title>"));
    assert_eq!(scoped.matches("url(#state-x-paint)").count(), 5);
    assert!(scoped.contains("href=\"#state-x-paint\""));
}

#[test]
fn cross_state_paths_avoid_intervening_cards_and_describe_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"}],"configuration":{"root_module":{"resources":[{"address":"test.b","expressions":{"input":{"references":["test.a.id"]}}}]}}}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    graph.0.components.push(crate::model::SyntheticComponent {
        entity: crate::model::ArchitectureEntity::synthetic(
            "test",
            "group",
            graph
                .nodes
                .iter()
                .flat_map(|n| n.entity.provenance.clone())
                .collect(),
        ),
        id: "test-group".into(),
        kind: "test".into(),
        label: "Test group".into(),
        members: vec!["test.a".into(), "test.b".into()],
    });
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let local = Layout::new(&graph);
    assert!(local.bounds[0].right() < local.bounds[1].right());
    let (_, height) = super::super::dimensions(&graph, &local);
    let index = super::super::relationship_index::Index::new(&graph, &local);
    let offset = 100
        + super::super::legend::height(&graph)
        + super::super::annotation_area(&graph, &index, super::super::AddressFormat::Qualified)
            .height;
    let obstacles: Vec<_> = [offset, offset + height + 60]
        .into_iter()
        .flat_map(|offset| {
            local.bounds.iter().map(move |b| {
                let mut b = *b;
                b.origin.y += offset;
                b
            })
        })
        .collect();
    let endpoint_y = offset + local.bounds[0].origin.y + local.header_heights[0] / 2;
    let mut architecture = crate::model::architecture::Architecture {
        states: BTreeMap::from([(first.clone(), graph.clone()), (second.clone(), graph)]),
        relationships: vec![crate::model::architecture::CrossStateEdge {
            from: Endpoint {
                state: first,
                address: "test.a".into(),
            },
            to: Endpoint {
                state: second,
                address: "test.a".into(),
            },
            provenance: crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.x".into(),
                output: "value".into(),
                output_action: None,
            },
        }],
    };
    let svg = render_states(
        &crate::view::apply(&architecture, &Default::default()).unwrap(),
        super::super::AddressFormat::Qualified,
    );
    assert!(svg.contains("2 named states with 1 cross-state dependencies"));
    let line = svg
        .lines()
        .find(|l| l.contains("data-cross-state=\"true\""))
        .unwrap();
    let path = line
        .split(" d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let tokens: Vec<_> = path.split_whitespace().collect();
    let points: Vec<Point> = tokens
        .chunks_exact(3)
        .map(|p| Point {
            x: p[1].parse().unwrap(),
            y: p[2].parse().unwrap(),
        })
        .collect();
    assert!(points.len() > 4);
    assert_eq!(points[0].y, endpoint_y);
    assert_eq!(points.last().unwrap().y, endpoint_y + height + 60);
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in &obstacles {
            let crosses = if a.x == b.x {
                a.x > obstacle.origin.x
                    && a.x < obstacle.right()
                    && a.y.max(b.y) > obstacle.origin.y
                    && a.y.min(b.y) < obstacle.origin.y + obstacle.height
            } else {
                a.y > obstacle.origin.y
                    && a.y < obstacle.origin.y + obstacle.height
                    && a.x.max(b.x) > obstacle.origin.x
                    && a.x.min(b.x) < obstacle.right()
            };
            assert!(!crosses, "{segment:?} crosses {obstacle:?}");
        }
    }
    let mut parallel = architecture.relationships[0].clone();
    let crate::model::architecture::CrossStateProvenance::TerraformRemoteState { output, .. } =
        &mut parallel.provenance;
    *output = "second_value".into();
    architecture.relationships.push(parallel);
    let svg = render_states(
        &crate::view::apply(&architecture, &Default::default()).unwrap(),
        super::super::AddressFormat::Qualified,
    );
    let paths: std::collections::BTreeSet<_> = svg
        .lines()
        .filter(|line| line.contains("data-cross-state=\"true\""))
        .map(|line| {
            line.split(" d=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
        })
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(svg.contains("outputs.second_value"));
    assert!(svg.contains("outputs.value"));
    architecture.relationships.reverse();
    assert_eq!(
        svg,
        render_states(
            &crate::view::apply(&architecture, &Default::default()).unwrap(),
            super::super::AddressFormat::Qualified
        )
    );
}

#[test]
fn cross_state_obstacles_only_relax_the_endpoint_and_its_ancestors() {
    let rectangle = |x, y, width, height| Bounds {
        origin: Point { x, y },
        width,
        height,
    };
    let state = StateObstacles {
        scopes: Vec::new(),
        markers: vec![rectangle(270, 219, 24, 18)],
        bounds: vec![
            rectangle(50, 100, 400, 300),
            rectangle(80, 180, 150, 96),
            rectangle(500, 120, 300, 300),
        ],
        headers: vec![60, 96, 60],
        parents: vec![None, Some(0), None],
    };
    let obstacles = state.for_endpoint(1);
    assert_eq!(obstacles[0], state.bounds[0].header(60));
    assert_eq!(obstacles[2], state.bounds[2]);
    assert_eq!(obstacles[3], state.markers[0]);
    let route = crate::layout::route_to_margin(
        Point { x: 230, y: 228 },
        Point { x: 900, y: 228 },
        &obstacles,
    );
    for unrelated in [state.bounds[2], state.markers[0]] {
        for segment in route.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            let crosses = if a.x == b.x {
                a.x > unrelated.origin.x
                    && a.x < unrelated.right()
                    && a.y.max(b.y) > unrelated.origin.y
                    && a.y.min(b.y) < unrelated.origin.y + unrelated.height
            } else {
                a.y > unrelated.origin.y
                    && a.y < unrelated.origin.y + unrelated.height
                    && a.x.max(b.x) > unrelated.origin.x
                    && a.x.min(b.x) < unrelated.right()
            };
            assert!(!crosses, "{segment:?}");
        }
    }
}
