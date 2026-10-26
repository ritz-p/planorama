use crate::{model::EdgeKind, plan, semantic, view};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/docdb-plan.json");

fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}

fn placed(v: &Value) -> bool {
    let g = graph(v);
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.from].address == "aws_vpc.main"
            && g.nodes[e.to].address == "aws_docdb_cluster.main"
    })
}

fn owned(v: &Value) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == "aws_docdb_cluster_instance.member")
        .unwrap()
        .member_owner
        .is_some()
}

#[test]
fn database_cluster_network_and_membership_are_independent() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(placed(&v));
    assert!(owned(&v));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 9);
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == "aws_docdb_subnet_group")
            .count(),
        1
    );
    assert!(!g.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && g.nodes[e.to].resource_type == "aws_docdb_cluster_instance"));
    v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"] = json!(["unknown-sg"]);
    assert!(!placed(&v));
    assert!(owned(&v));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Cluster membership"));
    }
}

#[test]
fn database_membership_requires_current_unique_typed_known_identifier() {
    for case in [
        "literal",
        "wrong-output",
        "mismatch",
        "unknown",
        "source-unknown",
        "wrong-type",
        "wrong-provider",
        "deleted",
        "deposed",
        "data",
        "scope",
        "ambiguous",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["cluster_identifier"] =
                    json!({"constant_value":"cluster-main"})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_docdb_cluster.main.id"]})
            }
            "mismatch" => {
                v["resource_changes"][7]["change"]["after"]["cluster_identifier"] = json!("other")
            }
            "unknown" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"cluster_identifier":true})
            }
            "source-unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"cluster_identifier":true})
            }
            "wrong-type" => v["resource_changes"][6]["type"] = json!("aws_rds_cluster"),
            "wrong-provider" => {
                v["resource_changes"][6]["provider_name"] = json!("example.org/foreign/aws")
            }
            "deleted" => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][6]["deposed"] = json!("old"),
            "data" => v["resource_changes"][6]["mode"] = json!("data"),
            "scope" => {
                v["configuration"]["provider_config"]["aws_other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
                    json!("aws_other");
            }
            "ambiguous" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["cluster_identifier"] = json!({"references":["aws_docdb_cluster.main.cluster_identifier", "aws_docdb_cluster.other.cluster_identifier"]})
            }
            _ => unreachable!(),
        }
        assert!(!owned(&v), "{case}");
    }
}

#[test]
fn database_network_requires_complete_common_vpc_and_compatible_group() {
    for case in [
        "extra-subnet",
        "unknown-subnet",
        "extra-sg",
        "unknown-sg",
        "mixed-vpc",
        "group-name",
        "group-output",
        "literal-subnets",
        "deleted-subnet",
        "deposed-group",
        "data-subnet",
        "unknown-vpc",
        "group-region",
        "wrong-provider",
        "ambiguous-group",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "extra-subnet" => {
                v["resource_changes"][5]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "subnet-b", "missing"])
            }
            "unknown-subnet" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"subnet_ids":[false,true]})
            }
            "extra-sg" => {
                v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"] =
                    json!(["sg-broker", "missing"])
            }
            "unknown-sg" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"vpc_security_group_ids":true})
            }
            "mixed-vpc" => {
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            "group-name" => {
                v["resource_changes"][6]["change"]["after"]["db_subnet_group_name"] =
                    json!("different")
            }
            "group-output" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["db_subnet_group_name"] =
                    json!({"references":["aws_docdb_subnet_group.shared.arn"]})
            }
            "literal-subnets" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["subnet_ids"] =
                    json!({"constant_value":["subnet-a","subnet-b"]})
            }
            "deleted-subnet" => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            "deposed-group" => v["resource_changes"][5]["deposed"] = json!("old"),
            "data-subnet" => v["resource_changes"][2]["mode"] = json!("data"),
            "unknown-vpc" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true})
            }
            "group-region" => {
                v["configuration"]["provider_config"]["aws_other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                    json!("aws_other")
            }
            "wrong-provider" => {
                v["resource_changes"][5]["provider_name"] = json!("example.org/foreign/aws")
            }
            "ambiguous-group" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["db_subnet_group_name"] =
                    json!({"references":["var.dynamic"]})
            }
            _ => unreachable!(),
        }
        assert!(!placed(&v), "{case}");
    }
}

#[test]
fn database_filtering_and_named_state_preserve_identity_without_borrowing_proof() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][7]["previous_address"] = json!("aws_docdb_cluster_instance.old");
    v["resource_changes"][7]["change"]["actions"] = json!(["update"]);
    let mut old = v["resource_changes"][7].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for source in &parsed.nodes {
        let nodes = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
        assert_eq!(nodes[0].metadata, source.metadata);
        if source.deposed_key.is_some() {
            assert!(nodes[0].member_owner.is_none());
        }
    }
    let selected = view::single(
        &graph(&serde_json::from_str(FIXTURE).unwrap()),
        &view::Options {
            focus: Some("aws_docdb_cluster_instance.member".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment)
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(6);
    assert!(!owned(&missing));
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
            format: crate::OutputFormat::Html,
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

#[test]
fn database_account_scope_and_filtered_network_proof_are_preserved() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    for node in &graph.nodes {
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012",
            "{}",
            node.address
        );
    }
    let output = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert!(
        doc.descendants()
            .any(|n| n.attribute("data-account") == Some("123456789012")
                && n.attribute("data-region") == Some("us-east-1"))
    );
    let groups = crate::view::scope::groups(&graph, &crate::layout::ContainmentTree::new(&graph));
    assert!(groups.keys().all(Option::is_some));
    for depth in [0, 1, 2] {
        let selected = view::single(
            &graph,
            &view::Options {
                focus: Some("aws_docdb_cluster.main".into()),
                depth: Some(depth),
                ..Default::default()
            },
        )
        .unwrap();
        let placement = selected.edges.iter().any(|e| {
            e.kind == EdgeKind::Containment
                && selected.nodes[e.to].address == "aws_docdb_cluster.main"
        });
        assert_eq!(placement, depth == 2, "focus depth {depth}");
    }
}
