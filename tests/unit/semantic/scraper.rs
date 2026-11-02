use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/prometheus-scraper-plan.json")).unwrap()
}
fn after<'a>(input: &'a mut Value, kind: &str) -> &'a mut Value {
    &mut input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["type"] == kind)
        .unwrap()["change"]["after"]
}
fn expressions<'a>(input: &'a mut Value, kind: &str) -> &'a mut Value {
    &mut input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["address"] == format!("{kind}.main"))
        .unwrap()["expressions"]
}

#[test]
fn scraper_variants_and_network_proof_keep_destination_independent() {
    for variant in ["eks", "vpc"] {
        for case in 0..7 {
            let mut input = fixture();
            if variant == "vpc" {
                let mut block =
                    after(&mut input, "aws_prometheus_scraper")["source"][0]["eks"].take();
                block[0].as_object_mut().unwrap().remove("cluster_arn");
                after(&mut input, "aws_prometheus_scraper")["source"][0]["vpc"] = block;
                let mut block =
                    expressions(&mut input, "aws_prometheus_scraper")["source"][0]["eks"].take();
                block[0].as_object_mut().unwrap().remove("cluster_arn");
                expressions(&mut input, "aws_prometheus_scraper")["source"][0]["vpc"] = block;
            }
            match case {
                0 => {}
                1 => {
                    after(&mut input, "aws_prometheus_scraper")["source"][0]
                        [if variant == "eks" { "vpc" } else { "eks" }] = json!([{}])
                }
                2 => {
                    after(&mut input, "aws_prometheus_scraper")["source"][0][variant][0]["subnet_ids"] =
                        json!(["subnet-main", "missing"])
                }
                3 => after(&mut input, "aws_security_group")["vpc_id"] = json!("other-vpc"),
                4 => {
                    after(&mut input, "aws_prometheus_scraper")["destination"][0]["cloudwatch"] =
                        json!([{"dataset_arn":"default"}])
                }
                5 => {
                    expressions(&mut input, "aws_prometheus_scraper")["source"][0][variant][0]["security_group_ids"] =
                        json!({"constant_value":["sg-main"]})
                }
                _ => {
                    after(&mut input, "aws_prometheus_scraper")["source"][0][variant][0]["subnet_ids"] =
                        json!(["subnet-main"])
                }
            }
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            let parents = graph
                .edges
                .iter()
                .filter(|e| {
                    e.kind == EdgeKind::Containment
                        && graph.nodes[e.to].resource_type == "aws_prometheus_scraper"
                })
                .map(|e| graph.nodes[e.from].resource_type.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                parents,
                if case == 0 { vec!["aws_vpc"] } else { vec![] },
                "{variant} case {case}"
            );
            assert!(graph.edges.iter().all(|e| e.kind != EdgeKind::Containment
                || graph.nodes[e.to].resource_type != "aws_prometheus_workspace"));
            assert_eq!(graph.coverage.selected.resources, 10);
        }
    }
}

#[test]
fn scraper_references_reject_wrong_types_providers_unknowns_and_historical_targets() {
    for kind in [
        "aws_eks_cluster",
        "aws_prometheus_workspace",
        "aws_iam_role",
    ] {
        for case in 0..6 {
            let mut input = fixture();
            let node = input["resource_changes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|n| n["type"] == kind)
                .unwrap();
            match case {
                0 => node["provider_name"] = json!("registry.terraform.io/other/aws"),
                1 => node["change"]["after_unknown"] = json!({"arn":true}),
                2 => node["change"]["after"]["arn"] = json!("wrong"),
                3 => node["change"]["actions"] = json!(["delete"]),
                4 => node["deposed"] = json!("old"),
                _ => node["mode"] = json!("data"),
            }
            let parsed = plan::parse(&input.to_string()).unwrap();
            let reference = match kind {
                "aws_eks_cluster" => "source.eks.cluster_arn",
                "aws_prometheus_workspace" => "destination.amp.workspace_arn",
                _ => "role_configuration.source_role_arn",
            };
            assert!(
                parsed
                    .attributes
                    .iter()
                    .filter(|r| r.attribute == reference)
                    .all(|r| !r.service_reference_verified),
                "{kind} case {case}"
            );
        }
    }
}

#[test]
fn scraper_payload_and_computed_role_are_not_emitted() {
    let input = include_str!("../../fixtures/prometheus-scraper-plan.json");
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let result = crate::generate(
            input,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!result.content.contains("PRIVATE_"));
    }
}

#[test]
fn scraper_cross_account_destination_requires_both_configured_roles() {
    for complete in [true, false] {
        let mut input = fixture();
        let workspace = "arn:aws:aps:us-west-2:999999999999:workspace/ws-example";
        let role = "arn:aws:iam::999999999999:role/target";
        after(&mut input, "aws_prometheus_workspace")["arn"] = json!(workspace);
        after(&mut input, "aws_prometheus_scraper")["destination"][0]["amp"][0]["workspace_arn"] =
            json!(workspace);
        after(&mut input, "aws_prometheus_scraper")["role_configuration"][0]["target_role_arn"] =
            json!(role);
        expressions(&mut input, "aws_prometheus_scraper")["role_configuration"][0]["target_role_arn"] =
            json!({"references":["aws_iam_role.target.arn"]});
        input["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_iam_role.target","type":"aws_iam_role","mode":"managed","change":{"actions":["create"],"after":{"arn":role}}}));
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":"aws_iam_role.target","provider_config_key":"aws"}));
        if !complete {
            expressions(&mut input, "aws_prometheus_scraper")["role_configuration"][0]
                .as_object_mut()
                .unwrap()
                .remove("source_role_arn");
        }
        let mut parsed = plan::parse(&input.to_string()).unwrap();
        for node in &mut parsed.nodes {
            node.provider_configuration.as_mut().unwrap().account = AwsAccountScope::new(
                if node.resource_type == "aws_prometheus_workspace"
                    || node.address == "aws_iam_role.target"
                {
                    "999999999999"
                } else {
                    "123456789012"
                },
            );
        }
        let evidence = parsed
            .nodes
            .iter()
            .map(|node| {
                let n = input["resource_changes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["address"].as_str() == Some(&node.address))
                    .unwrap();
                (&n["change"]["after"], &n["change"]["after_unknown"])
            })
            .collect::<Vec<_>>();
        super::annotate(&parsed.nodes, &mut parsed.attributes, &evidence);
        assert_eq!(
            parsed
                .attributes
                .iter()
                .any(|r| r.attribute == "destination.amp.workspace_arn"
                    && r.service_reference_verified),
            complete
        );
    }
}

#[test]
fn scraper_eks_network_requires_cluster_vpc_proof_and_logging_preserves_owner() {
    let mut input = fixture();
    expressions(&mut input, "aws_eks_cluster")["vpc_config"] = json!([]);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert!(graph.edges.iter().all(|e| e.kind != EdgeKind::Containment
        || graph.nodes[e.to].resource_type != "aws_prometheus_scraper"));
    let log = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_prometheus_scraper_logging_configuration")
        .unwrap();
    assert!(
        log.references
            .iter()
            .any(|r| r.label == "Prometheus scraper owner" && r.complete)
    );
    assert!(
        log.references
            .iter()
            .any(|r| r.label == "Scraper log group" && r.complete)
    );
}
