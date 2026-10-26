use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/msk-plan.json");

fn set_path(value: &mut Value, path: &[&str], replacement: Value) {
    if value.is_array() {
        set_path(&mut value[0], path, replacement);
    } else if let Some((field, rest)) = path.split_first() {
        set_path(&mut value[*field], rest, replacement);
    } else {
        *value = replacement;
    }
}

fn complete(value: &Value, index: usize, field: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let node = graph
        .nodes
        .iter()
        .find(|n| n.address == value["resource_changes"][index]["address"])
        .unwrap();
    node.references.iter().any(|r| {
        r.attribute == field
            && r.complete
            && super::label(&node.resource_type, field) == Some(r.label)
    })
}

fn parent(value: &Value, address: &str) -> Option<String> {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    graph
        .edges
        .iter()
        .find(|e| e.kind == EdgeKind::Containment && graph.nodes[e.to].address == address)
        .map(|e| graph.nodes[e.from].address.clone())
}

#[test]
fn msk_network_context_and_shared_associations_preserve_identity() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(
        parent(&v, "aws_msk_cluster.main").as_deref(),
        Some("aws_vpc.cluster")
    );
    assert_eq!(
        parent(&v, "aws_msk_serverless_cluster.serverless").as_deref(),
        Some("aws_vpc.cluster")
    );
    assert_eq!(
        parent(&v, "aws_msk_vpc_connection.client").as_deref(),
        Some("aws_vpc.client")
    );
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 15);
    for (index, fields) in [
        (
            10,
            vec![
                "broker_node_group_info.client_subnets",
                "broker_node_group_info.security_groups",
                "configuration_info.arn",
            ],
        ),
        (
            11,
            vec!["vpc_config.subnet_ids", "vpc_config.security_group_ids"],
        ),
        (12, vec!["cluster_arn", "secret_arn_list"]),
        (13, vec!["cluster_arn", "secret_arn"]),
        (
            14,
            vec![
                "target_cluster_arn",
                "vpc_id",
                "client_subnets",
                "security_groups",
            ],
        ),
    ] {
        for field in fields {
            assert!(complete(&v, index, field), "{index} {field}");
        }
    }
    assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_msk_vpc_connection")
            .unwrap()
            .placement,
        Placement::Spatial
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
        assert!(output.contains("SCRAM secrets"));
        assert!(output.contains("MSK configuration"));
    }
}

#[test]
fn msk_links_reject_incomplete_wrong_and_historical_evidence() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (index, field, endpoint) in [
        (10, "broker_node_group_info.client_subnets", 2),
        (10, "broker_node_group_info.security_groups", 5),
        (10, "configuration_info.arn", 7),
        (11, "vpc_config.subnet_ids", 2),
        (11, "vpc_config.security_group_ids", 5),
        (12, "cluster_arn", 10),
        (12, "secret_arn_list", 8),
        (13, "secret_arn", 8),
        (14, "target_cluster_arn", 10),
        (14, "vpc_id", 1),
        (14, "client_subnets", 4),
        (14, "security_groups", 6),
    ] {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "ambiguous",
            "wrong-type",
            "wrong-provider",
            "deleted",
            "deposed",
            "data",
            "scope",
            "mismatch",
            "wrong-field",
        ] {
            let mut v = fixture.clone();
            let path = field.split('.').collect::<Vec<_>>();
            let endpoint_address = v["resource_changes"][endpoint]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                "literal" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"constant_value":"PRIVATE_LITERAL"}),
                ),
                "unknown" => v["resource_changes"][index]["change"]["after_unknown"] = json!(true),
                "source-unknown" => {
                    v["resource_changes"][endpoint]["change"]["after_unknown"] = json!(true)
                }
                "ambiguous" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":["var.dynamic"]}),
                ),
                "wrong-type" => v["resource_changes"][endpoint]["type"] = json!("aws_s3_bucket"),
                "wrong-provider" => {
                    v["resource_changes"][endpoint]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "deleted" => {
                    v["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][endpoint]["deposed"] = json!("old"),
                "data" => v["resource_changes"][endpoint]["mode"] = json!("data"),
                "scope" => {
                    let mut config = v["configuration"]["provider_config"]["aws"].clone();
                    config["expressions"]["region"] = json!({"constant_value":"us-west-2"});
                    v["configuration"]["provider_config"]["aws.other"] = config;
                    v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                        json!("aws.other");
                }
                "mismatch" => set_path(
                    &mut v["resource_changes"][index]["change"]["after"],
                    &path,
                    json!("PRIVATE_OTHER"),
                ),
                "wrong-field" => set_path(
                    &mut v["configuration"]["root_module"]["resources"][index]["expressions"],
                    &path,
                    json!({"references":[format!("{endpoint_address}.wrong")]}),
                ),
                _ => unreachable!(),
            }
            assert!(!complete(&v, index, field), "{index} {field} {case}");
        }
    }
}

#[test]
fn msk_requires_complete_sets_revision_proof_and_common_client_vpc() {
    for bad in [
        json!([]),
        json!(["subnet-a", "subnet-a"]),
        json!(["subnet-a"]),
        json!(["subnet-a", "PRIVATE_EXTRA"]),
        json!(["subnet-a", null]),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][10]["change"]["after"]["broker_node_group_info"][0]["client_subnets"] =
            bad;
        assert!(parent(&v, "aws_msk_cluster.main").is_none());
    }
    for case in [
        "revision-mismatch",
        "revision-unknown",
        "revision-literal",
        "revision-other",
        "subnet-vpc",
        "sg-vpc",
        "client-vpc",
        "multi-vpc",
        "unknown-member",
        "missing-sg",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "revision-mismatch"=>v["resource_changes"][10]["change"]["after"]["configuration_info"][0]["revision"]=json!(2),
            "revision-unknown"=>v["resource_changes"][10]["change"]["after_unknown"]=json!({"configuration_info":[{"revision":true}]}),
            "revision-literal"=>v["configuration"]["root_module"]["resources"][10]["expressions"]["configuration_info"][0]["revision"]=json!({"constant_value":3}),
            "revision-other"=>v["configuration"]["root_module"]["resources"][10]["expressions"]["configuration_info"][0]["revision"]=json!({"references":["aws_msk_cluster.main.current_version"]}),
            "subnet-vpc"|"sg-vpc"=>{let i=if case=="subnet-vpc"{3}else{5};v["resource_changes"][i]["change"]["after"]["vpc_id"]=json!("vpc-client");v["configuration"]["root_module"]["resources"][i]["expressions"]["vpc_id"]=json!({"references":["aws_vpc.client.id"]});},
            "client-vpc"=>{v["resource_changes"][14]["change"]["after"]["vpc_id"]=json!("vpc-cluster");v["configuration"]["root_module"]["resources"][14]["expressions"]["vpc_id"]=json!({"references":["aws_vpc.cluster.id"]});},
            "multi-vpc"=>{let copy=v["resource_changes"][11]["change"]["after"]["vpc_config"][0].clone();v["resource_changes"][11]["change"]["after"]["vpc_config"].as_array_mut().unwrap().push(copy);},
            "unknown-member"=>v["resource_changes"][10]["change"]["after_unknown"]=json!({"broker_node_group_info":[{"client_subnets":[false,true]}]}),
            "missing-sg"=>v["configuration"]["root_module"]["resources"][10]["expressions"]["broker_node_group_info"][0].as_object_mut().unwrap().remove("security_groups").map(|_|()).unwrap(),
            _=>unreachable!(),
        }
        if case.starts_with("revision") {
            assert!(!complete(&v, 10, "configuration_info.arn"), "{case}");
        } else {
            let address = match case {
                "client-vpc" => "aws_msk_vpc_connection.client",
                "multi-vpc" => "aws_msk_serverless_cluster.serverless",
                _ => "aws_msk_cluster.main",
            };
            assert!(parent(&v, address).is_none(), "{case}");
            assert!(
                crate::provider::diagnostics(&plan::parse(&v.to_string()).unwrap())
                    .iter()
                    .any(|d| d.address == address),
                "{case}"
            );
        }
    }
}

#[test]
fn msk_selection_lifecycle_and_named_state_proofs_remain_local() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][12]["previous_address"] = json!("aws_msk_scram_secret_association.old");
    v["resource_changes"][12]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][12]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][12].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 16);
    for source in &parsed.nodes {
        let nodes = graph
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
    }
    let selected = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_msk_cluster.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        !selected
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment)
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(8);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
