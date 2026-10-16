use crate::{icons, model::*, plan, semantic};
use serde_json::{Value, json};

#[test]
fn eventbridge_preserves_current_deposed_move_import_and_actions() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/eventbridge-plan.json")).unwrap();
    input["resource_changes"][1]["previous_address"] = json!("aws_cloudwatch_event_rule.old");
    input["resource_changes"][1]["change"]["importing"] = json!({"id":"private-import"});
    let mut deposed = input["resource_changes"][1].clone();
    deposed["deposed"] = json!("old-key");
    deposed["change"]["actions"] = json!(["delete"]);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(deposed);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let rules: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.address == "aws_cloudwatch_event_rule.jobs")
        .collect();
    assert_eq!(rules.len(), 2);
    assert_ne!(rules[0].entity.id, rules[1].entity.id);
    assert!(
        rules
            .iter()
            .any(|n| n.action == Action::Delete && n.deposed_key.is_some())
    );
    let current = rules.iter().find(|n| n.deposed_key.is_none()).unwrap();
    assert_eq!(current.action, Action::Update);
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_cloudwatch_event_rule.old")
    );
    assert!(current.metadata.import.is_some());
}

#[test]
fn eventbridge_identity_scope_and_explicit_references_without_inferred_flow() {
    let input: Value =
        serde_json::from_str(include_str!("../../fixtures/eventbridge-plan.json")).unwrap();
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(graph.nodes.len(), 4);
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    for node in &graph.nodes {
        assert_eq!(node.role, ResourceRole::Node);
        assert_eq!(node.placement, Placement::Spatial);
        assert_eq!(
            icons::for_node(node),
            Some(icons::ResourceIcon::EventBridge)
        );
        assert!(node.entity.scope.as_ref().unwrap().region.is_some());
    }
    assert!(graph.nodes.iter().any(|n| n.mode == EntityMode::Data));
    let rule = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_cloudwatch_event_rule.jobs")
        .unwrap();
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.to == rule
                && graph.nodes[e.from].address == "aws_cloudwatch_event_bus.events")
    );
    for expression in [
        json!({"constant_value":"events"}),
        json!({"references":["aws_cloudwatch_event_bus.events[count.index].name"]}),
    ] {
        let mut input = input.clone();
        input["configuration"]["root_module"]["resources"][1]["expressions"]["event_bus_name"] =
            expression;
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert_eq!(graph.nodes.len(), 4);
        assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    }
    let mut foreign = input;
    foreign["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws");
    let graph = semantic::transform(&plan::parse(&foreign.to_string()).unwrap());
    let bus = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_cloudwatch_event_bus.events")
        .unwrap();
    assert_eq!(bus.role, ResourceRole::Unknown);
    assert!(icons::for_node(bus).is_none());
}
