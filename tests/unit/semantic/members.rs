use crate::{layout::Layout, model::EdgeKind, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/aurora-plan.json")).unwrap()
}

#[test]
fn verified_membership_survives_incompatible_presentation_scope() {
    let mut value = input();
    value["configuration"]["root_module"]["resources"][5]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let member = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_rds_cluster_instance.member[0]")
        .unwrap();
    assert!(member.member_owner.is_none());
    assert!(
        member
            .references
            .iter()
            .any(|reference| reference.attribute == "cluster_identifier"
                && reference.kind == crate::model::ReferenceKind::Membership)
    );
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let metadata = doc
        .descendants()
        .find(|node| node.attribute("id") == Some("resource-references"))
        .unwrap();
    let references: Vec<Value> = serde_json::from_str(metadata.text().unwrap()).unwrap();
    assert!(references.iter().any(
        |reference| reference["consumer"] == member.entity.id.as_str()
            && reference["attribute"] == "cluster_identifier"
            && reference["kind"] == "membership"
    ));
    assert!(
        references
            .iter()
            .any(|reference| reference["kind"] == "configured")
    );
}

#[test]
fn member_accounts_require_shared_configuration_or_verified_account() {
    for case in [
        "same_configuration",
        "unknown_accounts",
        "same_account",
        "different_accounts",
    ] {
        let mut value = input();
        if case != "same_configuration" {
            value["configuration"]["provider_config"]["aws.member"] =
                value["configuration"]["provider_config"]["aws"].clone();
            value["configuration"]["provider_config"]["aws.member"]["alias"] = json!("member");
            value["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                json!("aws.member");
        }
        if case == "different_accounts" {
            value["configuration"]["provider_config"]["aws.member"]["expressions"]["assume_role"]
                [0]["role_arn"]["constant_value"] = json!("arn:aws:iam::999999999999:role/Fixture");
        }
        let raw = if matches!(case, "same_account" | "different_accounts") {
            crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap()
        } else {
            plan::parse(&value.to_string()).unwrap()
        };
        let graph = semantic::transform(&raw);
        let expected = matches!(case, "same_configuration" | "same_account");
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.member_owner.is_some())
                .count(),
            if expected { 2 } else { 0 },
            "{case}"
        );
        let output = svg::render(&graph, &Layout::new(&graph));
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|node| node.attribute("data-embedded-member").is_some())
                .count(),
            if expected { 2 } else { 0 },
            "{case}"
        );
        assert_eq!(
            doc.descendants()
                .filter(|node| node.attribute("data-terraform-address").is_some())
                .count(),
            graph.nodes.len()
        );
    }
}

#[test]
fn deleting_members_use_before_identifiers() {
    for destroy_cluster in [false, true] {
        for matching in [false, true] {
            let mut value = input();
            value["resource_changes"][4]["change"]["before"] = json!({"id":"old-cluster"});
            value["resource_changes"][4]["change"]["after"] = json!({"id":"new-cluster"});
            if destroy_cluster {
                value["resource_changes"][4]["change"]["actions"] = json!(["delete"]);
                value["resource_changes"][4]["change"]["after"] = Value::Null;
            }
            for index in [5, 6] {
                value["resource_changes"][index]["change"] = json!({"actions":["delete"],"before":{"cluster_identifier": if matching {"old-cluster"} else {"new-cluster"}},"after":null});
            }
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .filter(|node| node.member_owner.is_some())
                    .count(),
                if matching { 2 } else { 0 }
            );
            let output = svg::render(&graph, &Layout::new(&graph));
            let doc = roxmltree::Document::parse(&output).unwrap();
            let members: Vec<_> = doc
                .descendants()
                .filter(|node| node.attribute("data-embedded-member").is_some())
                .collect();
            assert_eq!(members.len(), if matching { 2 } else { 0 });
            for member in members {
                assert!(
                    member
                        .children()
                        .find(|node| node.has_tag_name("title"))
                        .unwrap()
                        .text()
                        .unwrap()
                        .contains("; delete")
                );
            }
        }
    }
}

#[test]
fn local_logical_member_references_are_not_duplicated() {
    let mut value = input();
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_iam_policy.shared","type":"aws_iam_policy"}));
    value["configuration"]["root_module"]["resources"][5]["expressions"]["policy"] =
        json!({"references":["aws_iam_policy.shared.arn"]});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let policy = doc
        .descendants()
        .find(|node| node.attribute("data-terraform-address") == Some("aws_iam_policy.shared"))
        .unwrap();
    let href = format!("#{}", policy.attribute("id").unwrap());
    for member in doc
        .descendants()
        .filter(|node| node.attribute("data-embedded-member").is_some())
    {
        assert_eq!(
            member
                .descendants()
                .filter(|node| node.attribute("href") == Some(href.as_str()))
                .count(),
            1
        );
    }
}

#[test]
fn account_only_scopes_do_not_verify_regional_membership() {
    for overrides in [false, true] {
        let mut value = input();
        if overrides {
            for index in [4, 5] {
                value["configuration"]["root_module"]["resources"][index]["expressions"]["region"] =
                    json!({"references": ["var.region"]});
            }
        } else {
            value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                json!({"references": ["var.cluster_region"]});
            value["configuration"]["provider_config"]["aws.member"] =
                value["configuration"]["provider_config"]["aws"].clone();
            value["configuration"]["provider_config"]["aws.member"]["alias"] = json!("member");
            value["configuration"]["provider_config"]["aws.member"]["expressions"]["region"] =
                json!({"references": ["var.member_region"]});
            value["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                json!("aws.member");
        }
        let raw = crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        for node in graph.nodes.iter().filter(|node| {
            matches!(
                node.resource_type.as_str(),
                "aws_rds_cluster" | "aws_rds_cluster_instance"
            )
        }) {
            let scope = node.entity.scope.as_ref().unwrap();
            assert!(scope.account.is_some());
            assert!(scope.region.is_none());
            assert!(node.member_owner.is_none());
        }
    }
}

#[test]
fn indexed_cluster_requires_an_explicit_instance_reference() {
    let mut value = input();
    value["resource_changes"][4]["address"] = json!("aws_rds_cluster.main[0]");
    for explicit in [true, false] {
        value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] = json!({"references": [if explicit { "aws_rds_cluster.main[0].id" } else { "aws_rds_cluster.main.id" }]});
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.member_owner.is_some())
                .count(),
            if explicit { 2 } else { 0 }
        );
    }
}

#[test]
fn cluster_members_preserve_semantics_and_reserve_final_bounds() {
    let raw = crate::plan::evidence::tests::parse_fixture(&input().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let owner = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_rds_cluster")
        .unwrap();
    let layout = Layout::new(&graph);
    let output = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&output).unwrap();
    let mut children = Vec::new();
    for (index, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_rds_cluster_instance")
    {
        assert_eq!(view::routes::owner(&graph, node), Some(owner));
        assert!(node.entity.scope.as_ref().unwrap().account.is_some());
        assert_eq!(node.entity.scope, graph.nodes[owner].entity.scope);
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.to == index && e.kind == EdgeKind::Containment)
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|e| e.from == owner && e.to == index && e.kind == EdgeKind::Dependency)
        );
        let bounds = layout.bounds[index];
        let parent = layout.bounds[owner];
        assert!(bounds.origin.x > parent.origin.x && bounds.right() < parent.right());
        assert!(bounds.origin.y > parent.origin.y + layout.header_heights[owner]);
        assert!((bounds.origin.y + bounds.height) < (parent.origin.y + parent.height));
        let card = document
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some(&node.address))
            .unwrap();
        let rect = card.children().find(|n| n.has_tag_name("rect")).unwrap();
        assert_eq!(
            ["x", "y", "width", "height"].map(|key| rect
                .attribute(key)
                .unwrap()
                .parse::<usize>()
                .unwrap()),
            [
                bounds.origin.x,
                bounds.origin.y,
                bounds.width,
                bounds.height
            ]
        );
        children.push(bounds);
    }
    assert_eq!(children.len(), 2);
    assert!(
        (children[0].origin.y + children[0].height) < children[1].origin.y
            || (children[1].origin.y + children[1].height) < children[0].origin.y
    );
    assert!(output.contains("Referenced by: module.root.aws_instance.client"));
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        if edge.kind == EdgeKind::Dependency && edge.to == owner {
            assert!(path.len() >= 2);
            assert!(
                path.iter()
                    .all(|point| !children.iter().any(|b| point.x > b.origin.x
                        && point.x < b.right()
                        && point.y > b.origin.y
                        && point.y < (b.origin.y + b.height)))
            );
        }
    }
}

#[test]
fn uncertain_cluster_ownership_falls_back_without_hiding_instances() {
    for case in [
        "missing",
        "literal",
        "partial",
        "wrong_type",
        "ambiguous",
        "dynamic",
        "implicit_index",
        "foreign_owner",
        "foreign_child",
        "data_owner",
        "data_child",
        "scope",
        "unknown_scope",
        "dynamic_scope",
        "deposed",
        "identifier_mismatch",
        "identifier_unknown",
        "other_attribute",
    ] {
        let mut value = input();
        match case {
            "identifier_mismatch" | "identifier_unknown" => {
                for index in [5, 6] {
                    value["resource_changes"][index]["change"]["after"]["cluster_identifier"] =
                        json!("main-cluster-other");
                    if case == "identifier_unknown" {
                        value["resource_changes"][index]["change"]["after_unknown"] =
                            json!({"cluster_identifier":true});
                    }
                }
            }
            "other_attribute" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_rds_cluster.main.endpoint"]});
            }
            "missing" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"] = json!({})
            }
            "literal" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] =
                    json!({"constant_value":"cluster"})
            }
            "partial" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_rds_cluster.main.id", "var.unknown"]})
            }
            "wrong_type" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_subnet.a.id"]})
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["cluster_identifier"] =
                    json!({"references":["aws_rds_cluster.main.id", "aws_subnet.a.id"]})
            }
            "foreign_owner" => value["resource_changes"][4]["provider_name"] = json!("custom/aws"),
            "foreign_child" => {
                for i in [5, 6] {
                    value["resource_changes"][i]["provider_name"] = json!("custom/aws");
                }
            }
            "data_owner" => value["resource_changes"][4]["mode"] = json!("data"),
            "data_child" => {
                for i in [5, 6] {
                    value["resource_changes"][i]["mode"] = json!("data");
                }
            }
            "scope" => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "unknown_scope" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("region");
            }
            "dynamic_scope" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references": ["var.region"]});
            }
            "deposed" => {
                for i in [5, 6] {
                    value["resource_changes"][i]["deposed"] = json!("old");
                }
            }
            _ => {}
        }
        let mut raw = plan::parse(&value.to_string()).unwrap();
        for reference in &mut raw.attributes {
            if reference.attribute == "cluster_identifier" {
                if case == "dynamic" {
                    reference
                        .issues
                        .insert(crate::model::DiagnosticReason::DynamicInstanceSelection);
                }
                if case == "implicit_index" {
                    reference.instances_explicit = false;
                }
            }
        }
        let graph = semantic::transform(&raw);
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.resource_type == "aws_rds_cluster_instance")
                .count(),
            2
        );
        assert!(
            graph.nodes.iter().all(|n| n.member_owner.is_none()),
            "{case}"
        );
        if matches!(case, "unknown_scope" | "dynamic_scope") {
            assert!(graph.nodes.iter().all(|node| node.entity.scope.is_none()));
            let output = svg::render(&graph, &Layout::new(&graph));
            let document = roxmltree::Document::parse(&output).unwrap();
            assert_eq!(
                document
                    .descendants()
                    .filter(|node| node.attribute("data-terraform-address").is_some())
                    .count(),
                graph.nodes.len()
            );
            assert!(!output.contains("data-embedded-member"));
        }
    }
}

#[test]
fn cluster_filtering_releases_members_and_preserves_unique_counts() {
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    for options in [
        view::Options {
            changed_only: true,
            ..Default::default()
        },
        view::Options {
            focus: Some("aws_rds_cluster_instance.member[0]".into()),
            depth: Some(0),
            ..Default::default()
        },
    ] {
        let filtered = view::single(&graph, &options).unwrap();
        assert!(
            filtered
                .nodes
                .iter()
                .all(|n| n.resource_type == "aws_rds_cluster_instance")
        );
        assert!(
            filtered
                .nodes
                .iter()
                .all(|n| view::routes::owner(&filtered, n).is_none())
        );
        let layout = Layout::new(&filtered);
        assert!(layout.bounds.iter().all(|b| b.width > 0 && b.height > 0));
    }
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_rds_cluster_instance.member[0]".into()),
            depth: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        filtered
            .nodes
            .iter()
            .filter(|n| view::routes::owner(&filtered, n).is_some())
            .count(),
        1
    );
}
