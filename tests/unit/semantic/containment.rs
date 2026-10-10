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
    assert_eq!(architecture.nodes, crate::semantic::base_graph(&raw).nodes);
    assert_eq!(architecture.edges.len(), raw.edges.len());
    assert!(
        crate::semantic::base_graph(&raw)
            .edges
            .iter()
            .all(|e| e.kind == EdgeKind::Dependency)
    );
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
        assert_eq!(
            semantic::transform(&raw).0,
            crate::semantic::base_graph(&raw)
        );
    }
}

#[test]
fn unsupported_types_and_data_queries_keep_dependency_edges() {
    for scenario in 0..2 {
        let mut input = input();
        match scenario {
            0 => {
                input["resource_changes"][1]["type"] = json!("aws_iam_role");
                input["resource_changes"][2]["type"] = json!("aws_network_interface");
            }
            _ => {
                input["resource_changes"][1]["mode"] = json!("data");
                input["resource_changes"][2]["mode"] = json!("data");
            }
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(
            semantic::transform(&raw).0,
            crate::semantic::base_graph(&raw)
        );
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
    assert_eq!(
        semantic::transform(&raw).0,
        crate::semantic::base_graph(&raw)
    );
}

#[test]
fn explicit_network_scope_rules_require_static_single_endpoints() {
    for (child_type, attribute, parent_type) in [
        ("aws_nat_gateway", "subnet_id", "aws_subnet"),
        ("aws_vpc_endpoint", "vpc_id", "aws_vpc"),
        ("aws_route_table", "vpc_id", "aws_vpc"),
        ("aws_security_group", "vpc_id", "aws_vpc"),
        ("aws_network_acl", "vpc_id", "aws_vpc"),
        ("aws_lb_target_group", "vpc_id", "aws_vpc"),
    ] {
        for scenario in 0..10 {
            let mut value = json!({"format_version":"1.2","resource_changes":[
                {"address":format!("{parent_type}.a[0]"),"type":parent_type},
                {"address":format!("{parent_type}.b"),"type":parent_type},
                {"address":format!("{child_type}.child"),"type":child_type}
            ],"configuration":{"root_module":{"resources":[
                {"address":format!("{child_type}.child"),"expressions":{attribute:{"references":[format!("{parent_type}.a[0].id")]}}}
            ]}}});
            let expr = &mut value["configuration"]["root_module"]["resources"][0]["expressions"];
            match scenario {
                1 => *expr = json!({attribute:{"constant_value":"literal-id"}}),
                2 => {
                    *expr = json!({attribute:{"references":[format!("{parent_type}.a[0].id"),format!("{parent_type}.b.id")]}})
                }
                3 => *expr = json!({attribute:{"references":["var.missing"]}}),
                4 => *expr = json!({"tags":{"references":[format!("{parent_type}.a[0].id")]}}),
                5 => {
                    *expr = json!({attribute:{"references":[format!("{parent_type}.a[count.index].id")]}})
                }
                6 => value["resource_changes"][2]["provider_name"] = json!("acme/custom"),
                7 => value["resource_changes"][2]["mode"] = json!("data"),
                8 => value["resource_changes"][0]["type"] = json!("aws_iam_role"),
                9 => value["resource_changes"][0]["provider_name"] = json!("acme/custom"),
                _ => {}
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Containment)
                    .count(),
                usize::from(scenario == 0),
                "{child_type} scenario {scenario}"
            );
        }
    }
}

#[test]
fn target_group_vpc_reference_is_optional_but_invalid_references_are_diagnosed() {
    for scenario in 0..3 {
        let expressions = match scenario {
            0 => json!({"target_type":{"constant_value":"lambda"}}),
            1 => json!({"vpc_id":{"constant_value":"TOP_SECRET_LITERAL"}}),
            _ => json!({"vpc_id":{"references":["aws_vpc.main.id"]}}),
        };
        let input = json!({"format_version":"1.2","resource_changes":[
            {"address":"aws_vpc.main","type":"aws_vpc"},
            {"address":"aws_lb_target_group.main","type":"aws_lb_target_group"}
        ],"configuration":{"root_module":{"resources":[
            {"address":"aws_lb_target_group.main","expressions":expressions}
        ]}}});
        let raw = plan::parse(&input.to_string()).unwrap();
        let diagnostics = semantic::diagnostics::collect(&raw);
        assert_eq!(diagnostics.len(), usize::from(scenario == 1));
        if let Some(diagnostic) = diagnostics.first() {
            assert_eq!(diagnostic.attribute, "vpc_id");
            assert_eq!(
                diagnostic.reason,
                crate::model::DiagnosticReason::NoResourceReference
            );
        }
        assert!(!format!("{diagnostics:?}").contains("TOP_SECRET"));
        assert_eq!(
            semantic::transform(&raw)
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Containment)
                .count(),
            usize::from(scenario == 2)
        );
    }
}
