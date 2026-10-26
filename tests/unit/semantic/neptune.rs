use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/neptune-plan.json");

fn result(v: &Value) -> (bool, bool) {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let placed = g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.from].address == "aws_vpc.main"
            && g.nodes[e.to].address == "aws_neptune_cluster.main"
    });
    let member = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_neptune_cluster_instance.member")
        .unwrap();
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == member.address)
    );
    (placed, member.member_owner.is_some())
}

#[test]
fn neptune_membership_checks_optional_instance_group_separately_from_network() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(result(&v), (true, true));
    v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"] = json!(["unproven"]);
    assert_eq!(result(&v), (false, true));
    v["resource_changes"][7]["change"]["after"]["neptune_subnet_group_name"] = json!("different");
    assert_eq!(result(&v), (false, false));
    v["resource_changes"][7]["change"]["after"]
        .as_object_mut()
        .unwrap()
        .remove("neptune_subnet_group_name");
    v["configuration"]["root_module"]["resources"][7]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("neptune_subnet_group_name");
    assert_eq!(result(&v), (false, true));
    v["resource_changes"][7]["change"]["after_unknown"] = json!({"neptune_subnet_group_name":true});
    assert_eq!(result(&v), (false, false));
}

#[test]
fn neptune_rejects_adjacent_service_and_incomplete_network_contracts() {
    for case in [
        "cluster-type",
        "cluster-output",
        "member-unknown",
        "member-data",
        "member-deposed",
        "deleted-cluster",
        "wrong-provider",
        "group-type",
        "subnet-unknown",
        "extra-subnet",
        "extra-sg",
        "mixed-vpc",
        "literal-group",
        "region",
        "instance-group-literal",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "cluster-type" => v["resource_changes"][6]["type"] = json!("aws_docdb_cluster"),
            "cluster-output" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_neptune_cluster.main.id"]})
            }
            "member-unknown" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"cluster_identifier":true})
            }
            "member-data" => v["resource_changes"][7]["mode"] = json!("data"),
            "member-deposed" => v["resource_changes"][7]["deposed"] = json!("old"),
            "deleted-cluster" => v["resource_changes"][6]["change"]["actions"] = json!(["delete"]),
            "wrong-provider" => {
                v["resource_changes"][6]["provider_name"] = json!("example.org/foreign/aws")
            }
            "group-type" => v["resource_changes"][5]["type"] = json!("aws_docdb_subnet_group"),
            "subnet-unknown" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"subnet_ids":[true,false]})
            }
            "extra-subnet" => {
                v["resource_changes"][5]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "subnet-b", "unproven"])
            }
            "extra-sg" => {
                v["resource_changes"][6]["change"]["after"]["vpc_security_group_ids"] =
                    json!(["sg-broker", "unproven"])
            }
            "mixed-vpc" => {
                v["resource_changes"][4]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][4]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            "literal-group" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["neptune_subnet_group_name"] =
                    json!({"constant_value":"db-subnets"})
            }
            "instance-group-literal" => {
                v["configuration"]["root_module"]["resources"][7]["expressions"]["neptune_subnet_group_name"] =
                    json!({"constant_value":"db-subnets"})
            }
            "region" => {
                v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
                    json!("other");
            }
            _ => unreachable!(),
        }
        let (placed, owned) = result(&v);
        if matches!(
            case,
            "cluster-output"
                | "member-unknown"
                | "member-data"
                | "member-deposed"
                | "instance-group-literal"
        ) {
            assert!(placed, "{case}");
            assert!(!owned, "{case}");
        } else {
            assert!(!placed, "{case}");
            if !matches!(
                case,
                "subnet-unknown" | "extra-subnet" | "extra-sg" | "mixed-vpc"
            ) {
                assert!(!owned, "{case}");
            }
        }
    }
}

#[test]
fn neptune_preserves_shared_identity_and_named_state_output() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 9);
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == "aws_neptune_subnet_group")
            .count(),
        1
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(6);
    assert!(!result(&missing).1);
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

#[test]
fn neptune_account_scope_and_filtered_network_proof_are_preserved() {
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
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_neptune_cluster.main".into()),
                depth: Some(depth),
                ..Default::default()
            },
        )
        .unwrap();
        let placement = selected.edges.iter().any(|e| {
            e.kind == EdgeKind::Containment
                && selected.nodes[e.to].address == "aws_neptune_cluster.main"
        });
        assert_eq!(placement, depth == 2, "focus depth {depth}");
    }
}
