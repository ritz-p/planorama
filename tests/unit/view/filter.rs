use super::*;

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
