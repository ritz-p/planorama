use crate::{
    layout::Layout,
    model::{EdgeKind, Placement},
    plan, semantic, svg, view,
};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/efs-plan.json")).unwrap()
}

#[test]
fn access_point_ownership_requires_the_existing_service_proof_and_compatible_scope() {
    for case in [
        "valid",
        "literal",
        "partial",
        "ambiguous",
        "mismatch",
        "unknown_value",
        "wrong_type",
        "foreign",
        "data",
        "deposed",
        "deleted",
        "data_owner",
        "unknown_scope",
        "scope_mismatch",
        "unknown_account",
        "duplicate_attribute",
    ] {
        let mut value = input();
        match case {
            "literal" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["file_system_id"] =
                    json!({"constant_value":"fs-synthetic"})
            }
            "partial" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["file_system_id"] =
                    json!({"references":["aws_efs_file_system.shared.id", "var.other"]})
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["file_system_id"] =
                    json!({"references":["aws_efs_file_system.shared.id", "aws_subnet.app.id"]})
            }
            "mismatch" => {
                value["resource_changes"][2]["change"]["after"]["file_system_id"] =
                    json!("fs-other")
            }
            "unknown_value" => {
                value["resource_changes"][2]["change"]["after_unknown"]["file_system_id"] =
                    json!(true)
            }
            "wrong_type" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["file_system_id"] =
                    json!({"references":["aws_efs_mount_target.app.id"]})
            }
            "foreign" => value["resource_changes"][0]["provider_name"] = json!("example/aws"),
            "data" => value["resource_changes"][2]["mode"] = json!("data"),
            "data_owner" => value["resource_changes"][0]["mode"] = json!("data"),
            "deposed" => value["resource_changes"][2]["deposed"] = json!("old"),
            "deleted" => {
                value["resource_changes"][2]["change"]["before"] =
                    value["resource_changes"][2]["change"]["after"].clone();
                value["resource_changes"][2]["change"]["after"] = Value::Null;
                value["resource_changes"][2]["change"]["actions"] = json!(["delete"]);
            }
            "unknown_scope" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            "scope_mismatch" => {
                value["configuration"]["root_module"]["resources"][2]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "unknown_account" => {
                value["configuration"]["provider_config"]["aws.other"] =
                    value["configuration"]["provider_config"]["aws"].clone();
                value["configuration"]["provider_config"]["aws.other"]["alias"] = json!("other");
                value["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.other");
            }
            _ => {}
        }
        let mut raw = plan::parse(&value.to_string()).unwrap();
        if case == "duplicate_attribute" {
            let child = raw
                .nodes
                .iter()
                .position(|n| n.resource_type == "aws_efs_access_point")
                .unwrap();
            let reference = raw
                .attributes
                .iter()
                .find(|r| r.target == child && r.attribute == "file_system_id")
                .unwrap()
                .clone();
            raw.attributes.push(reference);
        }
        let graph = semantic::transform(&raw);
        let (index, child) = graph
            .nodes
            .iter()
            .enumerate()
            .find(|(_, n)| n.resource_type == "aws_efs_access_point")
            .unwrap();
        assert_eq!(child.placement, Placement::Spatial);
        assert_eq!(child.member_owner.is_some(), case == "valid", "{case}");
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.to == index && e.kind == EdgeKind::Containment),
            "{case}"
        );
        let layout = Layout::new(&graph);
        assert!(layout.bounds[index].width > 0, "{case}");
        let output = svg::render(&graph, &layout);
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-terraform-address") == Some(child.address.as_str()))
                .count(),
            1,
            "{case}"
        );
        assert!(!output.contains("PRIVATE_PATH"));
        assert!(!output.contains("12345"));
    }
}

#[test]
fn filtering_owners_does_not_restore_children_or_lose_selected_access_points() {
    let value = input().to_string().replace(
        "aws_efs_access_point.app",
        "module.child.aws_efs_access_point.app",
    );
    let graph = semantic::transform(&plan::parse(&value).unwrap());
    for options in [
        view::Options {
            focus: Some("module.child.aws_efs_access_point.app".into()),
            depth: Some(0),
            ..Default::default()
        },
        view::Options {
            modules: vec!["module.child".into()],
            ..Default::default()
        },
    ] {
        let selected = view::single(&graph, &options).unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert_eq!(selected.nodes[0].resource_type, "aws_efs_access_point");
        assert!(view::routes::owner(&selected, &selected.nodes[0]).is_none());
        assert!(Layout::new(&selected).bounds[0].width > 0);
    }
    let selected = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_efs_file_system.shared".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert!(view::routes::cards(&selected, 0).is_empty());
}

#[test]
fn multiple_access_points_share_one_file_system_without_becoming_mount_target_children() {
    let mut value = input();
    let mut second = value["resource_changes"][2].clone();
    second["address"] = json!("aws_efs_access_point.second");
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let mut config = value["configuration"]["root_module"]["resources"][2].clone();
    config["address"] = json!("aws_efs_access_point.second");
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let layout = Layout::new(&graph);
    let mut children = Vec::new();
    for (index, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_efs_access_point")
    {
        let parent = view::routes::owner(&graph, node).unwrap();
        assert_eq!(graph.nodes[parent].resource_type, "aws_efs_file_system");
        let bounds = layout.bounds[index];
        assert!(bounds.origin.y > layout.bounds[parent].origin.y);
        assert!(
            bounds.origin.y + bounds.height
                < layout.bounds[parent].origin.y + layout.bounds[parent].height
        );
        children.push(bounds);
    }
    assert_eq!(children.len(), 2);
    assert!(
        children[0].origin.y + children[0].height < children[1].origin.y
            || children[1].origin.y + children[1].height < children[0].origin.y
    );
    let mount = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_efs_mount_target")
        .unwrap();
    assert!(graph.edges.iter().any(|e| e.to == mount
        && e.kind == EdgeKind::Containment
        && graph.nodes[e.from].resource_type == "aws_subnet"));
}
