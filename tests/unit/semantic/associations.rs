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
    let edge = &architecture.edges[0];
    assert_eq!(edge.kind, EdgeKind::Association);
    assert_eq!(architecture.nodes[edge.from].address, "aws_subnet.private");
    assert_eq!(
        architecture.nodes[edge.to].address,
        "aws_route_table.private"
    );
    assert_eq!(architecture, semantic::transform(&raw));
    assert!(
        !svg::render(&architecture, &Layout::new(&architecture))
            .contains("<title>aws_route_table_association.private</title>")
    );
}

#[test]
fn lowering_preserves_every_action_in_relationship_and_legend() {
    use crate::model::Action;
    for (actions, expected, label, color) in [
        (vec!["create"], Action::Create, "create", "#047857"),
        (vec!["update"], Action::Update, "update", "#1d4ed8"),
        (
            vec!["delete", "create"],
            Action::Replace,
            "replace",
            "#c2410c",
        ),
        (vec!["delete"], Action::Delete, "delete", "#b91c1c"),
        (vec!["no-op"], Action::Unchanged, "unchanged", "#64748b"),
        (vec!["read"], Action::Read, "read", "#7e22ce"),
        (vec!["future"], Action::Other, "other", "#854d0e"),
    ] {
        let mut input = input();
        input["resource_changes"][2]["change"]["actions"] = json!(actions);
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        let change = graph.edges[0].change.as_ref().unwrap();
        assert_eq!(change.action, expected);
        assert_eq!(change.address, "aws_route_table_association.private");
        let image = svg::render(&graph, &Layout::new(&graph));
        let count = match expected {
            Action::Create => 3,
            _ => 1,
        };
        assert!(image.contains(&format!("{label} ({count})")));
        assert!(image.contains(&format!(
            "association; {label}: aws_route_table_association.private"
        )));
        assert!(image.contains(&format!(
            "data-edge-kind=\"association\" fill=\"none\" stroke=\"{color}\""
        )));
        assert!(image.contains(&format!("marker-end=\"url(#arrow-{label})\"")));
    }
}

#[test]
fn parallel_association_changes_remain_distinct() {
    let mut input = input();
    let mut change = input["resource_changes"][2].clone();
    change["address"] = json!("aws_route_table_association.second");
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(change);
    let mut config = input["configuration"]["root_module"]["resources"][2].clone();
    config["address"] = json!("aws_route_table_association.second");
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(graph.edges.len(), 2);
    assert_ne!(graph.edges[0].change, graph.edges[1].change);
    let image = svg::render(&graph, &Layout::new(&graph));
    assert!(image.contains("create (4)"));
    assert!(image.contains("4 resources (2 cards), 2 relationships"));
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

fn relationship_input(kind: &str) -> Value {
    let [(from_attr, from_type), (to_attr, to_type)] = rule(kind).unwrap();
    json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":format!("{from_type}.main"),"type":from_type},
            {"address":format!("{to_type}.main"),"type":to_type},
            {"address":format!("{kind}.main"),"type":kind,"change":{"actions":["create"]}}
        ],
        "configuration":{"root_module":{"resources":[{
            "address":format!("{kind}.main"),"expressions":{
                (from_attr):{"references":[format!("{from_type}.main.id")]},
                (to_attr):{"references":[format!("{to_type}.main.id")]}
            }
        }]}}
    })
}

fn assert_relationship_actions(kind: &str) {
    use crate::model::Action;
    for (actions, action) in [
        (vec!["create"], Action::Create),
        (vec!["update"], Action::Update),
        (vec!["delete"], Action::Delete),
        (vec!["delete", "create"], Action::Replace),
        (vec!["no-op"], Action::Unchanged),
    ] {
        let mut input = relationship_input(kind);
        input["resource_changes"][2]["change"]["actions"] = json!(actions);
        let raw = plan::parse(&input.to_string()).unwrap();
        let original = raw.clone();
        let graph = semantic::transform(&raw);
        assert_eq!(raw, original);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        assert_eq!(edge.kind, EdgeKind::Association);
        let [(.., from_type), (.., to_type)] = rule(kind).unwrap();
        assert_eq!(graph.nodes[edge.from].resource_type, from_type);
        assert_eq!(graph.nodes[edge.to].resource_type, to_type);
        assert_eq!(
            edge.change,
            Some(EdgeChange {
                previous_address: None,
                address: format!("{kind}.main"),
                action
            })
        );
        assert_eq!(graph, semantic::transform(&raw));
    }
}

#[test]
fn target_group_attachment_lowers_with_action_metadata() {
    assert_relationship_actions("aws_lb_target_group_attachment");
}

#[test]
fn endpoint_route_table_association_lowers_with_action_metadata() {
    assert_relationship_actions("aws_vpc_endpoint_route_table_association");
}

#[test]
fn new_relationships_preserve_helpers_when_lowering_is_uncertain() {
    for kind in [
        "aws_lb_target_group_attachment",
        "aws_vpc_endpoint_route_table_association",
    ] {
        for (attribute, endpoint_type) in rule(kind).unwrap() {
            for scenario in [
                "missing",
                "literal",
                "unresolved",
                "ambiguous",
                "wrong_type",
                "foreign_endpoint",
                "foreign_helper",
                "dynamic",
                "data",
                "extra_input",
                "consumer",
            ] {
                let mut input = relationship_input(kind);
                let expressions =
                    &mut input["configuration"]["root_module"]["resources"][0]["expressions"];
                match scenario {
                    "missing" => {
                        expressions.as_object_mut().unwrap().remove(attribute);
                    }
                    "literal" => expressions[attribute] = json!({"constant_value":"literal-id"}),
                    "unresolved" => {
                        expressions[attribute] =
                            json!({"references":[format!("{endpoint_type}.main.id"),"var.unknown"]})
                    }
                    "ambiguous" => {
                        expressions[attribute] = json!({"references":[format!("{endpoint_type}.main.id"),format!("{endpoint_type}.other.id")]})
                    }
                    "wrong_type" => {
                        expressions[attribute] = json!({"references":["terraform_data.extra.id"]})
                    }
                    _ => {}
                }
                if scenario == "ambiguous" {
                    input["resource_changes"].as_array_mut().unwrap().push(
                        json!({"address":format!("{endpoint_type}.other"),"type":endpoint_type}),
                    );
                }
                if matches!(scenario, "extra_input" | "consumer" | "wrong_type") {
                    input["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"address":"terraform_data.extra","type":"terraform_data"}));
                }
                match scenario {
                    "foreign_endpoint" => {
                        for resource in input["resource_changes"].as_array_mut().unwrap() {
                            if resource["type"] == endpoint_type { resource["provider_name"] = json!("registry.terraform.io/custom/aws"); }
                        }
                    }
                    "foreign_helper" => input["resource_changes"][2]["provider_name"] = json!("registry.terraform.io/custom/aws"),
                    "data" => input["resource_changes"][2]["mode"] = json!("data"),
                    "extra_input" => input["configuration"]["root_module"]["resources"][0]["depends_on"] = json!(["terraform_data.extra"]),
                    "consumer" => input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"terraform_data.extra","expressions":{"input":{"references":[format!("{kind}.main.id")]}}})),
                    _ => {}
                }
                let mut raw = plan::parse(&input.to_string()).unwrap();
                if scenario == "dynamic" {
                    raw.attributes
                        .iter_mut()
                        .find(|r| r.attribute == attribute)
                        .unwrap()
                        .issues
                        .insert(crate::model::DiagnosticReason::DynamicInstanceSelection);
                }
                assert_eq!(
                    semantic::transform(&raw).0,
                    raw.graph,
                    "{kind} {attribute} {scenario}"
                );
            }
        }
    }
}
