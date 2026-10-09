use super::*;
use crate::{
    layout::Layout,
    model::{Action, Edge, ResourceRole},
    plan, semantic, svg,
};
use serde_json::json;

#[test]
fn metadata_is_hidden_only_for_data_and_surviving_edges_are_remapped() {
    for resource_type in [
        "aws_region",
        "aws_partition",
        "aws_caller_identity",
        "aws_availability_zones",
        "aws_iam_policy_document",
    ] {
        let raw = plan::parse(&json!({
            "format_version": "1.2",
            "resource_changes": [
                {"address": format!("data.{resource_type}.meta"), "type": resource_type, "mode": "data"},
                {"address": format!("{resource_type}.managed"), "type": resource_type, "mode": "managed"},
                {"address": "aws_instance.app", "type": "aws_instance"}
            ]
        }).to_string()).unwrap();
        let mut graph = crate::semantic::base_graph(&raw).clone();
        let hidden = graph
            .nodes
            .iter()
            .position(|node| node.mode == EntityMode::Data)
            .unwrap();
        let managed = graph
            .nodes
            .iter()
            .position(|node| {
                node.resource_type == resource_type && node.mode == EntityMode::Managed
            })
            .unwrap();
        let app = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_instance")
            .unwrap();
        graph.edges = vec![
            Edge::from((hidden, app)),
            Edge::from((managed, app)),
            Edge::from((app, hidden)),
        ];
        let result = visible(graph);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
        let edge = &result.edges[0];
        assert_eq!(result.nodes[edge.from].resource_type, resource_type);
        assert_eq!(result.nodes[edge.to].resource_type, "aws_instance");
        assert!(
            result
                .nodes
                .iter()
                .all(|node| node.mode == EntityMode::Managed)
        );
    }
}

#[test]
fn infrastructure_data_keeps_roles_actions_and_external_container_styling() {
    let raw = plan::parse(include_str!("../../../examples/data-plan.json")).unwrap();
    let original = raw.clone();
    let architecture = semantic::transform(&raw);
    assert_eq!(raw, original);
    assert_eq!(architecture, semantic::transform(&raw));
    assert_eq!(architecture.nodes.len(), raw.nodes.len() - 5);
    let layout = Layout::new(&architecture);
    let output = svg::render(&architecture, &layout);
    for resource_type in ["aws_vpc", "aws_subnet", "aws_lb", "aws_security_group"] {
        let external = architecture
            .nodes
            .iter()
            .find(|node| node.resource_type == resource_type && node.mode == EntityMode::Data)
            .unwrap();
        assert_eq!(external.action, Action::Unchanged);
        assert!(output.contains(&external.address));
    }
    let vpc = architecture
        .nodes
        .iter()
        .position(|node| node.address == "data.aws_vpc.shared")
        .unwrap();
    let subnet = architecture
        .nodes
        .iter()
        .position(|node| node.address == "aws_subnet.app")
        .unwrap();
    assert_eq!(architecture.nodes[vpc].role, ResourceRole::Container);
    assert_eq!(layout.parents[subnet], Some(vpc));
    assert_eq!(output.matches("data-external=\"true\"").count(), 4);
    assert!(output.contains("data-mode=\"data\" stroke-dasharray=\"6 4\""));
    assert!(output.contains("data-mode=\"managed\""));
    assert!(output.contains("data-role=\"policy\""));
    assert!(output.contains("data-role=\"controller\""));
    assert!(!output.contains("data.aws_region.current"));
    assert_eq!(
        output,
        svg::render(&architecture, &Layout::new(&architecture))
    );
}

#[test]
fn unknown_data_is_visible_and_all_metadata_can_render_an_empty_diagram() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"data.custom_lookup.value","type":"custom_lookup","mode":"data"}]}"#).unwrap();
    assert_eq!(
        semantic::transform(&raw).0,
        crate::semantic::base_graph(&raw)
    );
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"data.aws_region.current","type":"aws_region","mode":"data"}]}"#).unwrap();
    let architecture = semantic::transform(&raw);
    assert!(architecture.nodes.is_empty());
    assert!(architecture.edges.is_empty());
    assert!(svg::render(&architecture, &Layout::new(&architecture)).contains("No resources"));
}
