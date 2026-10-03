use super::*;
use crate::{layout::Layout, plan, semantic, svg};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap()
}

#[test]
fn lowers_resolved_relationship_and_remaps_indices_without_mutating_raw() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let original = raw.clone();
    let architecture = semantic::transform(&raw);
    assert_eq!(raw, original);
    assert_eq!(raw.nodes.len(), 3);
    assert_eq!(architecture.nodes.len(), 2);
    assert_eq!(architecture.edges.len(), 1);
    let edge = architecture.edges[0];
    assert_eq!(edge.kind, EdgeKind::Association);
    assert_eq!(architecture.nodes[edge.from].address, "aws_subnet.private");
    assert_eq!(
        architecture.nodes[edge.to].address,
        "aws_route_table.private"
    );
    assert_eq!(architecture, semantic::transform(&raw));
    assert!(
        !svg::render(&architecture, &Layout::new(&architecture))
            .contains("aws_route_table_association.private")
    );
}

#[test]
fn unresolved_ambiguous_or_non_attribute_dependencies_preserve_original_node() {
    for expression in [
        json!({}),
        json!({"subnet_id":{"constant_value":"subnet-123"},"route_table_id":{"references":["aws_route_table.private.id"]}}),
        json!({"subnet_id":{"references":["aws_subnet.private.id","var.unresolved"]},"route_table_id":{"references":["aws_route_table.private.id"]}}),
        json!({"subnet_id":{"references":["aws_route_table.private.id"]},"route_table_id":{"references":["aws_route_table.private.id"]}}),
    ] {
        let mut input = input();
        input["configuration"]["root_module"]["resources"][2]["expressions"] = expression;
        input["configuration"]["root_module"]["resources"][2]["depends_on"] =
            json!(["aws_subnet.private", "aws_route_table.private"]);
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(semantic::transform(&raw).0, raw.graph);
    }
}

#[test]
fn extra_incoming_or_outgoing_edges_and_data_mode_are_preserved() {
    for scenario in 0..3 {
        let mut input = input();
        match scenario {
            0 => {
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"address":"test.extra","type":"test"}));
                input["configuration"]["root_module"]["resources"][2]["depends_on"] =
                    json!(["test.extra"]);
            }
            1 => {
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"address":"test.consumer","type":"test"}));
                input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"test.consumer","expressions":{"input":{"references":["aws_route_table_association.private.id"]}}}));
            }
            _ => input["resource_changes"][2]["mode"] = json!("data"),
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(semantic::transform(&raw).0, raw.graph);
    }
}

#[test]
fn expanded_resource_instances_remain_visible_when_endpoint_is_ambiguous() {
    let mut input = input();
    input["resource_changes"][0]["address"] = json!("aws_subnet.private[0]");
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_subnet.private[1]","type":"aws_subnet"}));
    let raw = plan::parse(&input.to_string()).unwrap();
    assert_eq!(semantic::transform(&raw).0, raw.graph);
}
