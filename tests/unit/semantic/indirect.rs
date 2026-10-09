use super::*;
use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/rds-subnet-group-plan.json")).unwrap()
}

#[test]
fn rds_inherits_single_or_common_parent_and_preserves_group_dependencies() {
    for single in [false, true] {
        let mut input = fixture();
        if single {
            input["configuration"]["root_module"]["resources"][2]["expressions"]["subnet_ids"] =
                json!({"references":["aws_subnet.a.id"]});
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        assert!(semantic::diagnostics::collect(&raw).is_empty());
        let graph = semantic::transform(&raw).0;
        assert_eq!(graph.nodes, crate::semantic::base_graph(&raw).nodes);
        for kind in ["aws_db_instance", "aws_rds_cluster"] {
            let target = graph
                .nodes
                .iter()
                .position(|n| n.resource_type == kind)
                .unwrap();
            let parents: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.to == target && e.kind == EdgeKind::Containment)
                .collect();
            assert_eq!(parents.len(), 1);
            assert_eq!(
                graph.nodes[parents[0].from].address,
                if single {
                    "aws_subnet.a"
                } else {
                    "aws_vpc.main"
                }
            );
            assert!(graph.edges.iter().any(|e| e.to == target
                && e.kind == EdgeKind::Dependency
                && graph.nodes[e.from].resource_type == "aws_db_subnet_group"));
        }
        for edge in &raw.edges {
            if raw.nodes[edge.to].resource_type == "aws_db_subnet_group" {
                assert!(graph.edges.contains(&edge.endpoints().into()));
            }
        }
    }
}

#[test]
fn diagnostics_distinguish_indirect_ancestry_from_second_hop_failures() {
    for scenario in ["disjoint", "unresolved", "literal", "wrong_type", "dynamic"] {
        let mut input = fixture();
        let resources = &mut input["configuration"]["root_module"]["resources"];
        match scenario {
            "disjoint" => resources[1]["expressions"] = json!({}),
            "unresolved" => {
                resources[2]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_subnet.a.id", "var.missing"]})
            }
            "literal" => {
                resources[2]["expressions"]["subnet_ids"] = json!({"constant_value":["subnet-123"]})
            }
            "wrong_type" => {
                resources[2]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_vpc.main.id"]})
            }
            _ => {}
        }
        let mut raw = plan::parse(&input.to_string()).unwrap();
        if scenario == "dynamic" {
            raw.attributes
                .iter_mut()
                .find(|r| r.attribute == "subnet_ids")
                .unwrap()
                .issues
                .insert(DiagnosticReason::DynamicInstanceSelection);
        }
        let diagnostics = semantic::diagnostics::collect(&raw);
        for kind in ["aws_db_instance", "aws_rds_cluster"] {
            let workload: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.address == format!("{kind}.main"))
                .collect();
            if scenario == "disjoint" {
                assert_eq!(workload.len(), 1);
                assert_eq!(workload[0].attribute, "db_subnet_group_name");
                assert_eq!(
                    workload[0].reason,
                    DiagnosticReason::AmbiguousContainmentParent
                );
            } else {
                assert!(workload.is_empty(), "{scenario}: {workload:?}");
                assert!(diagnostics.iter().any(
                    |d| d.address == "aws_db_subnet_group.main" && d.attribute == "subnet_ids"
                ));
            }
        }
    }
}

#[test]
fn uncertain_indirect_references_do_not_invent_a_parent() {
    for scenario in [
        "missing",
        "literal",
        "ambiguous",
        "partial",
        "subnet_literal",
        "foreign_group",
        "foreign_subnet",
        "foreign_workload",
        "disjoint",
        "dynamic_group",
        "dynamic_subnet",
    ] {
        let mut input = fixture();
        let resources = &mut input["configuration"]["root_module"]["resources"];
        match scenario {
            "missing" => {
                resources[3]["expressions"]["db_subnet_group_name"] =
                    json!({"references":["aws_db_subnet_group.missing.name"]})
            }
            "literal" => {
                resources[3]["expressions"]["db_subnet_group_name"] =
                    json!({"constant_value":"group"})
            }
            "ambiguous" => {
                resources[3]["expressions"]["db_subnet_group_name"] = json!({"references":["aws_db_subnet_group.main.name","aws_db_subnet_group.other.name"]})
            }
            "partial" => {
                resources[2]["expressions"]["subnet_ids"] =
                    json!({"references":["aws_subnet.a.id","var.missing"]})
            }
            "subnet_literal" => {
                resources[2]["expressions"]["subnet_ids"] = json!({"constant_value":["subnet-123"]})
            }
            "disjoint" => resources[1]["expressions"] = json!({}),
            _ => {}
        }
        if scenario == "ambiguous" {
            input["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(json!({"address":"aws_db_subnet_group.other", "type":"aws_db_subnet_group"}));
        }
        let foreign = match scenario {
            "foreign_group" => Some(3),
            "foreign_subnet" => Some(1),
            "foreign_workload" => Some(4),
            _ => None,
        };
        if let Some(index) = foreign {
            input["resource_changes"][index]["provider_name"] =
                json!("registry.terraform.io/custom/aws");
        }
        let mut raw = plan::parse(&input.to_string()).unwrap();
        if scenario.starts_with("dynamic") {
            let attribute = if scenario == "dynamic_group" {
                "db_subnet_group_name"
            } else {
                "subnet_ids"
            };
            for reference in &mut raw.attributes {
                if reference.attribute == attribute {
                    reference
                        .issues
                        .insert(DiagnosticReason::DynamicInstanceSelection);
                }
            }
        }
        let graph = semantic::transform(&raw).0;
        let target = graph
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_db_instance")
            .unwrap();
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.to == target && e.kind == EdgeKind::Containment),
            "{scenario}"
        );
    }
}
