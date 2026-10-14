use super::*;

fn route(id: &str, coords: &[(usize, usize)]) -> Route {
    Route {
        identity: json!({"source": id, "target": format!("{id}-target")}),
        path: coords.iter().map(|&(x, y)| Point { x, y }).collect(),
        stroke: 6,
        soft: vec![],
        rounded: true,
    }
}

fn report(edges: Vec<Route>) -> Value {
    Scene {
        edges,
        ..Scene::default()
    }
    .finish(1000, 1000)
}

#[test]
fn retraced_sections_count_visible_pair_geometry_once() {
    for transpose in [false, true] {
        for (peer_y, shared_source, metric, expected, findings) in [
            (100, false, "independent_overlap_length", 400, 1),
            (110, false, "parallel_deficit", 2016, 1),
            (100, true, "shared_trunk_length", 400, 0),
        ] {
            let mut a = route("a", &[(0, 100), (400, 100), (100, 100)]);
            let mut b = route(
                if shared_source { "a" } else { "b" },
                &[(0, peer_y), (400, peer_y)],
            );
            b.identity["target"] = json!("peer-target");
            if transpose {
                for p in a.path.iter_mut().chain(&mut b.path) {
                    std::mem::swap(&mut p.x, &mut p.y);
                }
            }
            let value = report(vec![a, b]);
            assert_eq!(
                value["summary"]["visible_straight_sections"][metric],
                expected
            );
            assert_eq!(value["findings"].as_array().unwrap().len(), findings);
            assert_eq!(value["summary"]["raw_polyline"]["length"], 1100);
            assert_eq!(value["summary"]["raw_polyline"]["bends"], 1);
        }
    }
}

#[test]
fn separates_crossings_parallel_and_independent_overlap() {
    let value = report(vec![
        route("a", &[(0, 100), (400, 100)]),
        route("b", &[(200, 0), (200, 400)]),
        route("c", &[(50, 110), (350, 110)]),
        route("d", &[(50, 100), (350, 100)]),
    ]);
    let categories: BTreeSet<_> = value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["category"].as_str().unwrap())
        .collect();
    assert_eq!(
        categories,
        BTreeSet::from(["crossing", "independent_overlap", "parallel_proximity"])
    );
    assert_eq!(
        value["summary"]["visible_straight_sections"]["crossings"],
        3
    );
}

#[test]
fn shared_trunks_are_distinct_from_independent_overlap() {
    let a = route("shared", &[(0, 100), (300, 100), (300, 300)]);
    let mut b = route("shared", &[(0, 100), (200, 100), (200, 400)]);
    b.identity["target"] = json!("other");
    let value = report(vec![a, b]);
    assert!(
        value["summary"]["visible_straight_sections"]["shared_trunk_length"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        value["summary"]["visible_straight_sections"]["independent_overlap_length"],
        0
    );
}

#[test]
fn masks_remove_hidden_crossings_and_border_findings_have_locations() {
    let frame = borders::Frame::card(Bounds {
        origin: Point { x: 100, y: 50 },
        width: 200,
        height: 200,
    });
    let mut a = route("a", &[(0, 100), (400, 100)]);
    a.soft.push(Occluder {
        bounds: frame.bounds,
        cost: 1,
        step: 2,
    });
    let b = route("b", &[(200, 0), (200, 400)]);
    let c = route("c", &[(100, 45), (300, 45)]);
    let value = Scene {
        edges: vec![a, b, c],
        frames: vec![(json!("card"), frame)],
        ..Scene::default()
    }
    .finish(1000, 1000);
    assert_eq!(
        value["summary"]["visible_straight_sections"]["crossings"],
        1
    );
    assert!(
        value["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["category"] == "visible_border_clearance" && !f["segment"].is_null())
    );
}

#[test]
fn edge_order_does_not_change_report() {
    let coords = [[(0, 100), (400, 100)], [(200, 0), (200, 400)]];
    assert_eq!(
        report(vec![route("a", &coords[0]), route("b", &coords[1])]),
        report(vec![route("b", &coords[1]), route("a", &coords[0])])
    );
}

#[test]
#[ignore = "explicit large-fixture diagnostics overhead measurement"]
fn collection_overhead() {
    let raw =
        crate::plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    let layout = Layout::new(&graph);
    for enabled in [false, true, false, true] {
        let start = std::time::Instant::now();
        let mut report = Value::Null;
        let svg = crate::svg::render_report(
            &graph,
            &layout,
            crate::svg::AddressFormat::Qualified,
            crate::svg::Content::Standard,
            enabled.then_some(&mut report),
        );
        eprintln!(
            "diagnostics={enabled} render_ms={:.3} svg_bytes={} report_bytes={}",
            start.elapsed().as_secs_f64() * 1000.0,
            svg.len(),
            if enabled { report.to_string().len() } else { 0 }
        );
        assert!(svg.starts_with("<svg"));
    }
}

#[test]
fn selected_logical_resources_do_not_become_spatial_geometry() {
    let raw = crate::plan::parse(include_str!("../../fixtures/viewer-plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    let node = graph
        .nodes
        .iter()
        .find(|n| n.placement == crate::model::Placement::Logical)
        .unwrap();
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some(node.address.clone()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let layout = Layout::new(&selected);
    let mut report = Value::Null;
    crate::svg::render_report(
        &selected,
        &layout,
        crate::svg::AddressFormat::Qualified,
        crate::svg::Content::Standard,
        Some(&mut report),
    );
    assert_eq!(report["states"][0]["cards"]["spatial"], 0);
    assert_eq!(report["states"][0]["cards"]["logical_resources"], 1);
    assert_eq!(report["summary"]["routed_paths"], 0);
    assert!(report["findings"].as_array().unwrap().is_empty());
}

#[test]
fn relationship_resources_distinguish_shared_endpoints_after_reordering() {
    let raw =
        crate::plan::parse(include_str!("../../fixtures/relationship-index-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    let mut expected: Vec<_> = (0..graph.edges.len())
        .map(|i| identity(&graph, i, Some("test")).to_string())
        .collect();
    expected.sort();
    graph.edges.reverse();
    let mut actual: Vec<_> = (0..graph.edges.len())
        .map(|i| identity(&graph, i, Some("test")).to_string())
        .collect();
    actual.sort();
    assert_eq!(actual, expected);
    for a in 0..graph.edges.len() {
        for b in a + 1..graph.edges.len() {
            if graph.edges[a].endpoints() == graph.edges[b].endpoints()
                && graph.edges[a].change.as_ref().map(|c| &c.address)
                    != graph.edges[b].change.as_ref().map(|c| &c.address)
            {
                assert_ne!(identity(&graph, a, None), identity(&graph, b, None));
            }
        }
    }
}
