use crate::{layout::Layout, model::EdgeKind, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/aurora-plan.json")).unwrap()
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
    assert!(output.contains("Referenced by: aws_instance.client"));
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
        "deposed",
    ] {
        let mut value = input();
        match case {
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
