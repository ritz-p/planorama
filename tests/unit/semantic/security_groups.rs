use super::*;
use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn unknown_non_id_traversals_cannot_prove_security_group_attachments() {
    for kind in [
        "aws_instance",
        "aws_lb",
        "aws_db_instance",
        "aws_rds_cluster",
        "aws_ecs_service",
        "aws_vpc_endpoint",
        "aws_lambda_function",
        "aws_eks_cluster",
        "aws_opensearch_domain",
    ] {
        for field in ["id", "vpc_id", "name", "id[0]"] {
            for alias in [false, true] {
                let traversal = format!("aws_security_group.a.{field}");
                let selected = if alias {
                    "local.attachment"
                } else {
                    &traversal
                };
                let mut input = fixture(kind, json!({"references":[selected]}));
                input["configuration"]["root_module"]["locals"] =
                    json!({"attachment":{"references":[traversal]}});
                input["resource_changes"][1]["change"]["after_unknown"]["vpc_id"] = json!(true);
                input["resource_changes"][1]["change"]["after_unknown"]["name"] = json!(true);
                let raw = plan::parse(&input.to_string()).unwrap();
                let graph = semantic::transform(&raw);
                assert_eq!(
                    graph
                        .edges
                        .iter()
                        .filter(|e| e.kind == EdgeKind::Connection)
                        .count(),
                    usize::from(field == "id"),
                    "{kind} {field} alias={alias}"
                );
            }
        }
    }
}

fn fixture(kind: &str, expression: Value) -> Value {
    let count = expression["references"].as_array().map_or(0, Vec::len);
    let expressions = attachment(kind, expression);
    let mut input = json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc"},
        {"address":"aws_security_group.a","type":"aws_security_group"},
        {"address":"aws_security_group.b","type":"aws_security_group"},
        {"address":format!("{kind}.app"),"type":kind}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_security_group.a","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}},
        {"address":"aws_security_group.b","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}},
        {"address":format!("{kind}.app"),"expressions":expressions}
    ]}}});
    for index in [1, 2] {
        input["resource_changes"][index]["change"]["after_unknown"]["id"] = json!(true);
    }
    let mask = json!(vec![true; count]);
    input["resource_changes"][3]["change"]["after_unknown"] = attachment(kind, mask);
    input
}

fn attachment(kind: &str, value: Value) -> Value {
    let attribute = attribute(kind).unwrap();
    match attribute.split_once('.') {
        Some((block, field)) => json!({block:[{field:value}]}),
        None => json!({attribute:value}),
    }
}

#[test]
fn nested_workload_attachments_do_not_borrow_block_or_sibling_references() {
    for (kind, block) in [
        ("aws_lambda_function", "vpc_config"),
        ("aws_eks_cluster", "vpc_config"),
        ("aws_opensearch_domain", "vpc_options"),
    ] {
        for scenario in 0..4 {
            let mut input = fixture(kind, json!({"references":["aws_security_group.a.id"]}));
            let expressions =
                &mut input["configuration"]["root_module"]["resources"][2]["expressions"][block];
            *expressions = match scenario {
                0 => json!({"references":["aws_security_group.a.id"]}),
                1 => json!([{"subnet_ids":{"references":["aws_security_group.a.id"]}}]),
                2 => json!([{"security_group_ids":{"references":["aws_security_group.a.id"]}},{}]),
                _ => json!({"security_group_ids":{"references":["aws_security_group.a.id"]}}),
            };
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Connection)
                    .count(),
                usize::from(scenario == 3),
                "{kind} scenario {scenario}"
            );
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Containment)
                    .count(),
                2
            );
        }
    }
}

#[test]
fn planned_collection_proof_rejects_literal_elements_and_missing_or_unknown_shapes() {
    for kind in [
        "aws_instance",
        "aws_lb",
        "aws_db_instance",
        "aws_rds_cluster",
        "aws_ecs_service",
        "aws_vpc_endpoint",
        "aws_lambda_function",
        "aws_eks_cluster",
        "aws_opensearch_domain",
    ] {
        for scenario in [
            "mixed",
            "same_length_literal",
            "missing",
            "unknown_length",
            "known_match",
            "known_mismatch",
        ] {
            let mut input = fixture(kind, json!({"references":["aws_security_group.a.id"]}));
            let (values, mask) = match scenario {
                "mixed" => (json!([null, "TOP_SECRET_LITERAL"]), json!([true, false])),
                "same_length_literal" => (json!(["TOP_SECRET_LITERAL"]), json!([false])),
                "known_match" => (json!(["sg-known"]), json!([false])),
                "known_mismatch" => (json!(["TOP_SECRET_LITERAL"]), json!([false])),
                "unknown_length" => (Value::Null, json!(true)),
                _ => (Value::Null, Value::Null),
            };
            if scenario.starts_with("known") {
                input["resource_changes"][1]["change"] = json!({"after":{"id":"sg-known"}});
            }
            input["resource_changes"][3]["change"] =
                json!({"after":attachment(kind, values),"after_unknown":attachment(kind, mask)});
            let raw = plan::parse(&input.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Connection)
                    .count(),
                usize::from(scenario == "known_match"),
                "{kind} {scenario}"
            );
            assert!(!format!("{raw:?}").contains("TOP_SECRET"));
            if scenario != "known_match" {
                assert!(
                    semantic::diagnostics::collect(&raw)
                        .iter()
                        .any(|d| d.attribute == attribute(kind).unwrap()
                            && d.reason == DiagnosticReason::PartialResourceProvenance)
                );
            }
        }
    }
}

#[test]
fn supported_families_keep_single_and_multiple_security_groups_and_containment() {
    for kind in [
        "aws_instance",
        "aws_lb",
        "aws_db_instance",
        "aws_rds_cluster",
        "aws_ecs_service",
        "aws_vpc_endpoint",
        "aws_lambda_function",
        "aws_eks_cluster",
        "aws_opensearch_domain",
    ] {
        for refs in [
            json!(["aws_security_group.a.id"]),
            json!(["aws_security_group.a.id", "aws_security_group.b.id"]),
        ] {
            let raw = plan::parse(&fixture(kind, json!({"references":refs})).to_string()).unwrap();
            let graph = semantic::transform(&raw).0;
            assert_eq!(graph.nodes, crate::semantic::base_graph(&raw).nodes);
            let connections: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Connection)
                .collect();
            assert_eq!(connections.len(), refs.as_array().unwrap().len());
            assert!(connections.iter().all(|e| graph.nodes[e.from].resource_type
                == "aws_security_group"
                && graph.nodes[e.to].resource_type == kind));
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == EdgeKind::Containment)
                    .count(),
                2
            );
            assert!(
                graph
                    .nodes
                    .iter()
                    .filter(|n| n.resource_type == "aws_security_group")
                    .all(|n| n.role == crate::model::ResourceRole::Policy)
            );
            assert_eq!(graph, semantic::transform(&raw).0);
            assert!(
                !semantic::diagnostics::collect(&raw)
                    .iter()
                    .any(|d| d.attribute == attribute(kind).unwrap())
            );
        }
    }
}

#[test]
fn unresolved_ambiguous_dynamic_foreign_and_unrelated_references_remain_dependencies() {
    for kind in [
        "aws_instance",
        "aws_lb",
        "aws_db_instance",
        "aws_rds_cluster",
        "aws_ecs_service",
        "aws_vpc_endpoint",
        "aws_lambda_function",
        "aws_eks_cluster",
        "aws_opensearch_domain",
    ] {
        for scenario in [
            "literal",
            "partial",
            "dynamic",
            "ambiguous",
            "wrong_type",
            "foreign_source",
            "foreign_target",
            "data_target",
            "tags",
        ] {
            let expression = match scenario {
                "literal" => json!({"constant_value":["sg-123"]}),
                "partial" => json!({"references":["aws_security_group.a.id","var.missing"]}),
                "dynamic" => json!({"references":["aws_security_group.a.id","each.key"]}),
                _ => json!({"references":["aws_security_group.a.id"]}),
            };
            let mut input = fixture(kind, expression);
            match scenario {
                "ambiguous" => {
                    input["resource_changes"][1]["address"] = json!("aws_security_group.a[0]");
                    input["resource_changes"].as_array_mut().unwrap().push(
                        json!({"address":"aws_security_group.a[1]","type":"aws_security_group"}),
                    );
                }
                "wrong_type" => input["resource_changes"][1]["type"] = json!("aws_instance"),
                "foreign_source" => {
                    input["resource_changes"][1]["provider_name"] =
                        json!("registry.terraform.io/custom/aws")
                }
                "foreign_target" => {
                    input["resource_changes"][3]["provider_name"] =
                        json!("registry.terraform.io/custom/aws")
                }
                "data_target" => input["resource_changes"][3]["mode"] = json!("data"),
                "tags" => {
                    input["configuration"]["root_module"]["resources"][2]["expressions"] = json!({"tags":{"references":["aws_security_group.a.id"]},"security_groups":{"constant_value":["name"]}})
                }
                _ => {}
            }
            let raw = plan::parse(&input.to_string()).unwrap();
            let graph = semantic::transform(&raw).0;
            assert!(
                !graph.edges.iter().any(|e| e.kind == EdgeKind::Connection),
                "{kind} {scenario}"
            );
            assert_eq!(graph.nodes, crate::semantic::base_graph(&raw).nodes);
            let target = graph
                .nodes
                .iter()
                .position(|n| n.address == format!("{kind}.app"))
                .unwrap();
            assert!(
                graph
                    .edges
                    .iter()
                    .filter(|e| e.to == target)
                    .all(|e| e.kind == EdgeKind::Dependency)
            );
        }
    }
}

#[test]
fn explicit_indexed_and_data_security_groups_remain_individually_inspectable() {
    let mut input = fixture(
        "aws_instance",
        json!({"references":["aws_security_group.a[0].id","data.aws_security_group.b.id"]}),
    );
    input["resource_changes"][1]["address"] = json!("aws_security_group.a[0]");
    input["resource_changes"][2]["address"] = json!("data.aws_security_group.b");
    input["resource_changes"][2]["mode"] = json!("data");
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_security_group.a[1]","type":"aws_security_group"}));
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw).0;
    let sources: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Connection)
        .map(|e| graph.nodes[e.from].address.as_str())
        .collect();
    assert_eq!(
        sources,
        ["aws_security_group.a[0]", "data.aws_security_group.b"]
    );
}
