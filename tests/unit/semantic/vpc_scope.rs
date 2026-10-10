use crate::{
    model::{EdgeKind, RelationshipProvenance},
    plan, semantic,
};
use serde_json::{Value, json};

#[test]
fn subnet_groups_require_complete_subnets_with_one_unambiguous_vpc() {
    for kind in ["aws_db_subnet_group", "aws_elasticache_subnet_group"] {
        for scenario in 0..9 {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/subnet-groups-plan.json"))
                    .unwrap();
            let address = format!("{kind}.main");
            let resources = value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap();
            let group = resources
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            match scenario {
                1 => group["expressions"]["subnet_ids"] = json!({"references":["aws_subnet.a.id"]}),
                2 => {
                    group["expressions"]["subnet_ids"] =
                        json!({"constant_value":["subnet-literal"]})
                }
                3 => {
                    group["expressions"]["subnet_ids"] =
                        json!({"references":["aws_subnet.a.id", "var.missing"]})
                }
                4 => {
                    group["expressions"]["subnet_ids"] =
                        json!({"references":["aws_subnet.a[count.index].id"]})
                }
                5 => {
                    resources[1]["expressions"]["vpc_id"] =
                        json!({"references":["aws_vpc.other.id"]});
                    value["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"address":"aws_vpc.other","type":"aws_vpc"}));
                }
                6 => resources[1]["expressions"] = json!({}),
                7 => value["resource_changes"][1]["provider_name"] = json!("acme/custom"),
                8 => {
                    resources[0]["expressions"]["vpc_id"] =
                        json!({"references":["aws_vpc.main.id", "var.missing"]})
                }
                _ => {}
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            let target = graph
                .nodes
                .iter()
                .position(|n| n.address == address)
                .unwrap();
            let containment: Vec<_> = graph
                .relationships
                .iter()
                .filter(|r| {
                    r.to == graph.nodes[target].entity.id && r.kind == EdgeKind::Containment
                })
                .collect();
            assert_eq!(
                containment.len(),
                usize::from(scenario < 2),
                "{kind} scenario {scenario}"
            );
            if let Some(relationship) = containment.first() {
                assert!(relationship.inferred);
                assert!(relationship.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Reference {from, to} if from.address == "aws_vpc.main" && to.address == "aws_subnet.a")));
                assert!(relationship.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Reference {from, to} if from.address == "aws_subnet.a" && to.address == address)));
            }
            for edge in raw
                .edges
                .iter()
                .filter(|e| raw.nodes[e.to].address == address)
            {
                assert!(graph.edges.iter().any(|e| graph.nodes[e.from].address
                    == raw.nodes[edge.from].address
                    && e.to == target
                    && e.kind == EdgeKind::Dependency));
            }
            assert_eq!(graph, semantic::transform(&raw));
        }
    }
}

#[test]
fn workloads_use_only_complete_nested_subnet_references() {
    for (kind, block) in [
        ("aws_lambda_function", "vpc_config"),
        ("aws_eks_cluster", "vpc_config"),
        ("aws_opensearch_domain", "vpc_options"),
    ] {
        for scenario in 0..12 {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/vpc-workloads-plan.json"))
                    .unwrap();
            let address = format!("{kind}.main");
            let resources = value["configuration"]["root_module"]["resources"]
                .as_array_mut()
                .unwrap();
            let resource = resources
                .iter_mut()
                .find(|r| r["address"] == address)
                .unwrap();
            let config = &mut resource["expressions"][block];
            match scenario {
                1 => config[0]["subnet_ids"] = json!({"references":["aws_subnet.a.id"]}),
                2 => config[0]["subnet_ids"] = json!({"constant_value":["subnet-literal"]}),
                3 => {
                    config[0]["subnet_ids"] =
                        json!({"references":["aws_subnet.a.id", "var.missing"]})
                }
                4 => {
                    config[0]["subnet_ids"] = json!({"references":["aws_subnet.a[count.index].id"]})
                }
                5 => {
                    resources[1]["expressions"]["vpc_id"] =
                        json!({"references":["aws_vpc.other.id"]});
                    value["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"address":"aws_vpc.other","type":"aws_vpc"}));
                }
                6 => *config = json!({"references":["aws_subnet.a.id"]}),
                7 => *config = json!([{"security_group_ids":{"references":["aws_subnet.a.id"]}}]),
                8 => *config = json!([{"subnet_ids":{"references":["aws_subnet.a.id"]}},{}]),
                9 => value["resource_changes"][1]["provider_name"] = json!("acme/custom"),
                10 => *config = config[0].clone(),
                11 => resources[0]["expressions"] = json!({}),
                _ => {}
            }
            let raw = plan::parse(&value.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            let target = graph
                .nodes
                .iter()
                .position(|n| n.address == address)
                .unwrap();
            let parents: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.to == target && e.kind == EdgeKind::Containment)
                .collect();
            assert_eq!(
                parents.len(),
                usize::from(matches!(scenario, 0 | 1 | 10)),
                "{kind} scenario {scenario}"
            );
            if let Some(edge) = parents.first() {
                assert_eq!(graph.nodes[edge.from].address, "aws_vpc.main");
                assert!(graph.derived_containment(edge.from, target));
            }
            for edge in raw
                .edges
                .iter()
                .filter(|e| raw.nodes[e.to].address == address)
            {
                assert!(graph.edges.iter().any(|e| graph.nodes[e.from].address
                    == raw.nodes[edge.from].address
                    && e.to == target
                    && e.kind == EdgeKind::Dependency));
            }
            assert_eq!(graph, semantic::transform(&raw));
        }
    }
}
