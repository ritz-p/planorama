use super::*;
use crate::{plan, semantic};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/containment-plan.json")).unwrap()
}

#[test]
fn infers_only_explicit_parent_attributes_and_preserves_raw_graph() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let original = raw.clone();
    let architecture = semantic::transform(&raw);
    assert_eq!(raw, original);
    assert_eq!(architecture.nodes, raw.nodes);
    assert_eq!(architecture.edges.len(), raw.edges.len());
    assert!(raw.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    for edge in &architecture.edges {
        let types = (
            architecture.nodes[edge.from].resource_type.as_str(),
            architecture.nodes[edge.to].resource_type.as_str(),
        );
        let expected = match types {
            ("aws_vpc", "aws_subnet") | ("aws_subnet", "aws_instance") => EdgeKind::Containment,
            _ => EdgeKind::Dependency,
        };
        assert_eq!(edge.kind, expected);
    }
    assert_eq!(architecture, semantic::transform(&raw));
}

#[test]
fn module_inputs_support_containment_without_changing_other_references() {
    let raw = plan::parse(include_str!("../../../examples/plan.json")).unwrap();
    let architecture = semantic::transform(&raw);
    assert_eq!(
        architecture
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Containment)
            .count(),
        3
    );
    for edge in &architecture.edges {
        if architecture.nodes[edge.from].resource_type == "aws_ami" {
            assert_eq!(edge.kind, EdgeKind::Dependency);
        }
    }
}

#[test]
fn tags_depends_on_constants_and_partial_references_are_not_containment() {
    for scenario in 0..4 {
        let mut input = input();
        let resources = &mut input["configuration"]["root_module"]["resources"];
        for (target, parent, attribute) in [
            (1, "aws_vpc.main", "vpc_id"),
            (2, "aws_subnet.private", "subnet_id"),
        ] {
            resources[target]["depends_on"] = json!([parent]);
            resources[target]["expressions"] = match scenario {
                0 => json!({"tags":{"references":[format!("{parent}.id")]}}),
                1 => json!({}),
                2 => json!({attribute:{"constant_value":"external-id"}}),
                _ => json!({attribute:{"references":[format!("{parent}.id"), "var.unresolved"]}}),
            };
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(semantic::transform(&raw).0, raw.graph);
    }
}

#[test]
fn unsupported_types_and_data_queries_keep_dependency_edges() {
    for scenario in 0..2 {
        let mut input = input();
        match scenario {
            0 => {
                input["resource_changes"][1]["type"] = json!("aws_security_group");
                input["resource_changes"][2]["type"] = json!("aws_network_interface");
            }
            _ => {
                input["resource_changes"][1]["mode"] = json!("data");
                input["resource_changes"][2]["mode"] = json!("data");
            }
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(semantic::transform(&raw).0, raw.graph);
    }
}

#[test]
fn data_source_parent_can_contain_a_managed_child() {
    let mut input = input();
    input["resource_changes"][0]["mode"] = json!("data");
    let raw = plan::parse(&input.to_string()).unwrap();
    assert_eq!(
        semantic::transform(&raw)
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Containment)
            .count(),
        2
    );
}

#[test]
fn multiple_possible_parents_preserve_all_original_dependencies() {
    let mut input = input();
    input["resource_changes"][0]["address"] = json!("aws_vpc.main[0]");
    input["resource_changes"][1]["address"] = json!("aws_subnet.private[0]");
    input["resource_changes"].as_array_mut().unwrap().extend([
        json!({"address":"aws_vpc.main[1]","type":"aws_vpc"}),
        json!({"address":"aws_subnet.private[1]","type":"aws_subnet"}),
    ]);
    let raw = plan::parse(&input.to_string()).unwrap();
    assert_eq!(semantic::transform(&raw).0, raw.graph);
}
