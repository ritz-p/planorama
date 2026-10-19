use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/kms-plan.json")).unwrap()
}

#[test]
fn alias_embedding_requires_unique_service_evidence_and_compatible_scope() {
    for case in [
        "valid",
        "literal",
        "partial",
        "ambiguous",
        "mismatch",
        "unknown_value",
        "external",
        "data",
        "data_owner",
        "deposed",
        "deposed_owner",
        "deleted",
        "duplicate_attribute",
        "unknown_scope",
        "scope_mismatch",
        "unknown_account",
    ] {
        let mut value = input();
        match case {
            "literal" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["target_key_id"] =
                    json!({"constant_value":"key-fixture"})
            }
            "partial" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["target_key_id"] =
                    json!({"references":["aws_kms_key.data.key_id", "var.other"]})
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["target_key_id"] =
                    json!({"references":["aws_kms_key.data.key_id", "aws_s3_bucket.data.id"]})
            }
            "external" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["target_key_id"] =
                    json!({"references":["aws_kms_key.external.id"]})
            }
            "mismatch" => {
                value["resource_changes"][1]["change"]["after"]["target_key_id"] = json!("other")
            }
            "unknown_value" => {
                value["resource_changes"][1]["change"]["after_unknown"]["target_key_id"] =
                    json!(true)
            }
            "data" => value["resource_changes"][1]["mode"] = json!("data"),
            "data_owner" => value["resource_changes"][0]["mode"] = json!("data"),
            "deposed" => value["resource_changes"][1]["deposed"] = json!("old"),
            "deposed_owner" => value["resource_changes"][0]["deposed"] = json!("old"),
            "deleted" => {
                value["resource_changes"][1]["change"]["before"] =
                    value["resource_changes"][1]["change"]["after"].clone();
                value["resource_changes"][1]["change"]["after"] = Value::Null;
                value["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
            }
            _ => {}
        }
        let mut raw = semantic::Input::new(&plan::parse(&value.to_string()).unwrap());
        let child = raw
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_kms_alias")
            .unwrap();
        let parent = raw
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_kms_key")
            .unwrap();
        match case {
            "duplicate_attribute" => raw.attributes.push(
                raw.attributes
                    .iter()
                    .find(|r| r.target == child && r.attribute == "target_key_id")
                    .unwrap()
                    .clone(),
            ),
            "unknown_scope" => {
                raw.graph.nodes[child].entity.scope = None;
                raw.graph.nodes[parent].entity.scope = None;
            }
            "scope_mismatch" => raw.graph.nodes[child].entity.scope.as_mut().unwrap().global = true,
            "unknown_account" => {
                raw.graph.nodes[child]
                    .entity
                    .scope
                    .as_mut()
                    .unwrap()
                    .account = None;
                raw.graph.nodes[child].provider_configuration = None;
            }
            _ => {}
        }
        let graph = crate::provider::transform(raw);
        let (index, child) = graph
            .nodes
            .iter()
            .enumerate()
            .find(|(_, n)| n.resource_type == "aws_kms_alias")
            .unwrap();
        assert_eq!(child.placement, Placement::Spatial);
        assert_eq!(child.member_owner.is_some(), case == "valid", "{case}");
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
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.to == index && e.kind == crate::model::EdgeKind::Containment)
        );
    }
}

#[test]
fn selected_aliases_fall_back_and_shared_keys_are_counted_once() {
    let value = input()
        .to_string()
        .replace("aws_kms_alias.data", "module.child.aws_kms_alias.data");
    let graph = semantic::transform(&plan::parse(&value).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    let key = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_kms_key")
        .unwrap();
    assert!(key.member_owner.is_none());
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.label == "Encryption key reference" && r.complete)
            .count(),
        4
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_s3_bucket_server_side_encryption_configuration")
            .unwrap()
            .placement,
        Placement::Logical
    );
    for options in [
        view::Options {
            focus: Some("module.child.aws_kms_alias.data".into()),
            depth: Some(0),
            ..Default::default()
        },
        view::Options {
            modules: vec!["module.child".into()],
            ..Default::default()
        },
        view::Options {
            focus: Some("aws_kms_key.data".into()),
            depth: Some(0),
            ..Default::default()
        },
    ] {
        let selected = view::single(&graph, &options).unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert_eq!(selected.coverage.selected.resources, 1);
        assert!(view::routes::owner(&selected, &selected.nodes[0]).is_none());
        assert!(Layout::new(&selected).bounds[0].width > 0);
    }
}

#[test]
fn alias_identity_measurement_and_state_local_exports_survive_embedding() {
    let mut value = input();
    let address = format!("aws_kms_alias.data[\"{}<&\"]", "long".repeat(70));
    value["resource_changes"][1]["address"] = json!(address);
    value["resource_changes"][1]["previous_address"] = json!("aws_kms_alias.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 8);
    let (child, node) = graph
        .nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.resource_type == "aws_kms_alias" && n.deposed_key.is_none())
        .unwrap();
    let parent = view::routes::owner(&graph, node).unwrap();
    let layout = Layout::new(&graph);
    let bounds = layout.bounds[child];
    let frame = layout.bounds[parent];
    assert!(bounds.origin.x > frame.origin.x && bounds.right() < frame.right());
    assert!(
        bounds.origin.y > frame.origin.y
            && bounds.origin.y + bounds.height < frame.origin.y + frame.height
    );
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.member_owner.is_none())
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
    let text = &html[html.find("id=\"viewer-data\"").unwrap()..];
    let resources: Vec<Value> =
        serde_json::from_str(&text[text.find('>').unwrap() + 1..text.find("</script>").unwrap()])
            .unwrap();
    assert_eq!(resources.len(), 16);
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
            .find(|r| r["state"] == state && r["address"] == "aws_kms_key.data")
            .unwrap();
        assert_eq!(child["context"], json!([parent["id"]]));
        assert_ne!(child["id"], parent["id"]);
    }
}
