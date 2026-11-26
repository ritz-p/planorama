use super::*;

#[test]
fn cache_context_is_retained_only_for_selected_placement_evidence() {
    for group_selected in [false, true] {
        let mut input: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/elasticache-plan.json")).unwrap();
        let cache = "aws_elasticache_replication_group.main";
        let group = "aws_elasticache_subnet_group.main";
        for change in input["resource_changes"].as_array_mut().unwrap() {
            let address = change["address"].as_str().unwrap();
            let changed = address == cache || (group_selected && address == group);
            change["change"]["actions"] =
                serde_json::json!([if changed { "update" } else { "no-op" }]);
        }
        let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
        let filtered = single(
            &graph,
            &Options {
                changed_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        let mut expected = BTreeSet::from([cache]);
        if group_selected {
            expected.extend([group, "aws_vpc.main"]);
        }
        let addresses: BTreeSet<_> = filtered.nodes.iter().map(|n| n.address.as_str()).collect();
        assert_eq!(addresses, expected);
        assert_eq!(
            filtered
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Containment)
                .count(),
            if group_selected { 2 } else { 0 }
        );
    }
}

#[test]
fn current_lowered_relationship_and_deposed_node_make_focus_ambiguous() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    let mut deposed = input["resource_changes"][2].clone();
    deposed["deposed"] = serde_json::json!("previous");
    deposed["change"]["actions"] = serde_json::json!(["delete"]);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    let plan = crate::plan::parse(&input.to_string()).unwrap();
    let graph = crate::semantic::transform(&plan);
    let address = "aws_route_table_association.private";
    assert_eq!(
        graph.nodes.iter().filter(|n| n.address == address).count(),
        1
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.change.as_ref().is_some_and(|c| c.address == address))
    );
    let error = single(
        &graph,
        &Options {
            focus: Some(address.into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.contains("ambiguous"));
}

#[test]
fn filtering_does_not_mutate_source_graph_or_plan() {
    let raw = crate::plan::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/containment-plan.json"
    )))
    .unwrap();
    let original = raw.clone();
    let graph = crate::semantic::transform(&raw);
    let before = graph.clone();
    single(
        &graph,
        &Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(graph, before);
    assert_eq!(raw, original);
}
