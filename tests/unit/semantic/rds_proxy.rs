use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/rds-proxy-plan.json");

#[test]
fn rds_proxy_validates_owner_and_distinguishes_database_target_types() {
    super::super::service_references::tests::check_service_fixture(
        FIXTURE,
        "Proxy owner",
        "db_proxy_name",
    );
    for (source, target, label, attribute) in [
        (2, 4, "DB instance target", "db_instance_identifier"),
        (3, 5, "DB cluster target", "db_cluster_identifier"),
        (1, 4, "Target group", "target_group_name"),
    ] {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .swap(0, source);
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .swap(1, target);
        let list = input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap();
        list.swap(0, source);
        list.swap(1, target);
        super::super::service_references::tests::check_service_fixture(
            &input.to_string(),
            label,
            attribute,
        );
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 12);
    assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.to].resource_type == "aws_db_proxy"
        && graph.nodes[e.from].resource_type == "aws_vpc"));
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.to].resource_type == "aws_db_proxy"
        && graph.nodes[e.from].resource_type != "aws_vpc"));
    assert!(graph.edges.iter().any(
        |e| e.kind == EdgeKind::Connection && graph.nodes[e.to].resource_type == "aws_db_proxy"
    ));
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Authentication secret reference" && r.complete)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
        assert!(output.contains("DB instance target"));
        assert!(output.contains("DB cluster target"));
    }
}

#[test]
fn rds_proxy_rejects_conflicting_target_types_and_foreign_group_owners() {
    for case in 0..5 {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => {
                input["resource_changes"][4]["change"]["after"]["db_cluster_identifier"] =
                    json!("fixture-cluster")
            }
            1 => {
                input["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"db_cluster_identifier":true})
            }
            2 => {
                input["configuration"]["root_module"]["resources"][4]["expressions"]["target_group_name"] =
                    json!({"constant_value":"default"})
            }
            3 => {
                input["resource_changes"][1]["change"]["after"]["db_proxy_name"] =
                    json!("different")
            }
            _ => {
                let mut other = input["resource_changes"][0].clone();
                other["address"] = json!("aws_db_proxy.other");
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
                input["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"address":"aws_db_proxy.other","provider_config_key":"aws"}));
                input["configuration"]["root_module"]["resources"][1]["expressions"]["db_proxy_name"] =
                    json!({"references":["aws_db_proxy.other.name"]});
            }
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert!(
            !graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_db_proxy_target.instance")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "DB instance target" && r.complete),
            "{case}"
        );
    }
}

#[test]
fn rds_proxy_network_rejects_unknown_partial_and_noncurrent_endpoints() {
    for (attribute, endpoint, kind) in [
        ("vpc_subnet_ids", 7, EdgeKind::Containment),
        ("vpc_security_group_ids", 9, EdgeKind::Connection),
    ] {
        for case in 0..8 {
            let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                1 => {
                    input["resource_changes"][endpoint]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                2 => input["resource_changes"][endpoint]["deposed"] = json!("old"),
                3 => {
                    input["resource_changes"][endpoint]["change"]["after_unknown"] =
                        json!({"id":true})
                }
                4 => {
                    input["resource_changes"][0]["change"]["after"][attribute] = json!(["literal"])
                }
                5 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => {
                    input["configuration"]["root_module"]["resources"][endpoint]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                7 => input["resource_changes"][endpoint]["mode"] = json!("data"),
                _ => {}
            }
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .any(|e| e.kind == kind && graph.nodes[e.to].resource_type == "aws_db_proxy"),
                case == 0,
                "{attribute}/{case}"
            );
        }
    }
}
