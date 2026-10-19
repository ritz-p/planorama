use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/secrets-manager-plan.json")).unwrap()
}

#[test]
fn versions_embed_only_with_verified_managed_secret_and_compatible_scope() {
    for case in [
        "managed",
        "data_version",
        "data_secret",
        "name",
        "external",
        "unknown_scope",
        "different_scope",
        "deposed",
        "deleted",
    ] {
        let mut value = input();
        match case {
            "data_version" => {
                let address = "data.aws_secretsmanager_secret_version.app";
                value["resource_changes"][1]["address"] = json!(address);
                value["resource_changes"][1]["mode"] = json!("data");
                value["resource_changes"][1]["change"]["actions"] = json!(["read"]);
                value["configuration"]["root_module"]["resources"][1]["address"] = json!(address);
                value["configuration"]["root_module"]["resources"][1]["mode"] = json!("data");
            }
            "data_secret" => {
                value["resource_changes"][3]["change"]["after"] =
                    value["resource_changes"][0]["change"]["after"].clone();
                value["configuration"]["root_module"]["resources"][1]["expressions"]["secret_id"] =
                    json!({"references":["data.aws_secretsmanager_secret.lookup.id"]});
            }
            "name" => {
                value["resource_changes"][1]["change"]["after"]["secret_id"] = json!("fixture");
                value["configuration"]["root_module"]["resources"][1]["expressions"]["secret_id"] =
                    json!({"references":["aws_secretsmanager_secret.app.name"]});
            }
            "external" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["secret_id"] =
                    json!({"references":["aws_secretsmanager_secret.missing.arn"]})
            }
            "unknown_scope" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            "different_scope" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "deposed" => value["resource_changes"][1]["deposed"] = json!("old"),
            "deleted" => {
                value["resource_changes"][1]["change"]["before"] =
                    value["resource_changes"][1]["change"]["after"].clone();
                value["resource_changes"][1]["change"]["after"] = Value::Null;
                value["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
            }
            _ => {}
        }
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert_eq!(graph.coverage.selected.resources, 4);
        let (index, child) = graph
            .nodes
            .iter()
            .enumerate()
            .find(|(_, n)| n.resource_type == "aws_secretsmanager_secret_version")
            .unwrap();
        assert_eq!(child.placement, Placement::Spatial);
        assert_eq!(
            child.member_owner.is_some(),
            matches!(case, "managed" | "data_version"),
            "{case}"
        );
        let secrets: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.resource_type == "aws_secretsmanager_secret")
            .collect();
        assert_eq!(secrets.len(), 2);
        assert!(
            secrets
                .iter()
                .all(|n| n.placement == Placement::Spatial && n.member_owner.is_none())
        );
        assert_ne!(secrets[0].entity.id, secrets[1].entity.id);
        assert!(!graph.edges.iter().any(|e| e.to == index
            && matches!(
                e.kind,
                crate::model::EdgeKind::Containment | crate::model::EdgeKind::Association
            )));
        let layout = Layout::new(&graph);
        assert!(layout.bounds[index].width > 0, "{case}");
        let output = svg::render(&graph, &layout);
        assert!(!output.contains("SYNTHETIC_PAYLOAD_NOT_FOR_OUTPUT"));
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-terraform-address") == Some(child.address.as_str()))
                .count(),
            1,
            "{case}"
        );
    }
}

#[test]
fn direct_version_focus_and_module_selection_do_not_restore_secret_or_siblings() {
    let value = input().to_string().replace(
        "aws_secretsmanager_secret_version.app",
        "module.child.aws_secretsmanager_secret_version.app",
    );
    let graph = semantic::transform(&plan::parse(&value).unwrap());
    for options in [
        view::Options {
            focus: Some("module.child.aws_secretsmanager_secret_version.app".into()),
            depth: Some(0),
            ..Default::default()
        },
        view::Options {
            modules: vec!["module.child".into()],
            ..Default::default()
        },
        view::Options {
            focus: Some("aws_secretsmanager_secret.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    ] {
        let selected = view::single(&graph, &options).unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert_eq!(selected.coverage.selected.resources, 1);
        assert!(view::routes::owner(&selected, &selected.nodes[0]).is_none());
        assert!(Layout::new(&selected).bounds[0].width > 0);
        let output = svg::render(&selected, &Layout::new(&selected));
        if selected.nodes[0].resource_type == "aws_secretsmanager_secret_version" {
            assert!(output.contains("Unavailable"));
        }
    }
}

#[test]
fn multiple_versions_keep_individual_measurements_and_state_local_identity() {
    let mut value = input();
    let address = format!(
        "aws_secretsmanager_secret_version.app[\"{}<&\"]",
        "long".repeat(70)
    );
    value["resource_changes"][1]["address"] = json!(address);
    value["resource_changes"][1]["previous_address"] =
        json!("aws_secretsmanager_secret_version.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut second = value["resource_changes"][1].clone();
    second["address"] = json!("aws_secretsmanager_secret_version.app[\"second\"]");
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 6);
    let layout = Layout::new(&graph);
    let mut bounds = Vec::new();
    for (index, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_secretsmanager_secret_version")
    {
        if node.deposed_key.is_some() {
            assert!(node.member_owner.is_none());
            continue;
        }
        let parent = view::routes::owner(&graph, node).unwrap();
        let frame = layout.bounds[parent];
        let child = layout.bounds[index];
        assert!(child.origin.x > frame.origin.x && child.right() < frame.right());
        assert!(
            child.origin.y > frame.origin.y
                && child.origin.y + child.height < frame.origin.y + frame.height
        );
        bounds.push(child);
    }
    assert_eq!(bounds.len(), 2);
    assert!(
        bounds[0].origin.y + bounds[0].height < bounds[1].origin.y
            || bounds[1].origin.y + bounds[1].height < bounds[0].origin.y
    );
    let architecture = crate::model::architecture::Architecture {
        states: [
            (crate::model::StateId::new("one").unwrap(), graph.clone()),
            (crate::model::StateId::new("two").unwrap(), graph),
        ]
        .into(),
        relationships: Vec::new(),
    };
    let selected = view::apply(&architecture, &Default::default()).unwrap();
    let html = crate::html::render_states(&selected, svg::Options::default());
    assert!(!html.contains("PRIVATE_IMPORT"));
    assert!(!html.contains("SYNTHETIC_PAYLOAD_NOT_FOR_OUTPUT"));
    let text = &html[html.find("id=\"viewer-data\"").unwrap()..];
    let resources: Vec<Value> =
        serde_json::from_str(&text[text.find('>').unwrap() + 1..text.find("</script>").unwrap()])
            .unwrap();
    assert_eq!(resources.len(), 12);
    for state in ["one", "two"] {
        let child = resources
            .iter()
            .find(|r| {
                r["state"] == state
                    && r["address"] == address
                    && r["deposed"].as_str().is_none_or(str::is_empty)
            })
            .unwrap();
        let parent = resources
            .iter()
            .find(|r| r["state"] == state && r["address"] == "aws_secretsmanager_secret.app")
            .unwrap();
        assert_eq!(child["context"], json!([parent["id"]]));
    }
}
