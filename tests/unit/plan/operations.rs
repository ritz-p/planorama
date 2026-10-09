use super::*;
use serde_json::json;

#[test]
fn replacement_reasons_and_typed_paths_are_independent_of_actions_and_values() {
    use crate::model::{Action, AttributePathStep as Step};
    let raw =
        crate::plan::parse(include_str!("../../fixtures/replacement-reasons-plan.json")).unwrap();
    let node = |name: &str| {
        raw.nodes
            .iter()
            .find(|n| n.address == format!("terraform_data.{name}"))
            .unwrap()
    };
    assert_eq!(
        node("tainted").metadata.action_reason.as_deref(),
        Some("replace_because_tainted")
    );
    assert_eq!(node("tainted").action, Action::Replace);
    assert_eq!(
        node("paths").metadata.replace_paths,
        Some(vec![
            vec![Step::Attribute("subnet_id".into())],
            vec![
                Step::Attribute("network".into()),
                Step::Index(0),
                Step::Attribute("tags".into()),
                Step::Attribute("key.with[brackets]".into())
            ]
        ])
    );
    assert_eq!(
        node("future").metadata.action_reason.as_deref(),
        Some("future<&reason")
    );
    assert_eq!(node("unspecified").action, Action::Replace);
    assert_eq!(node("unspecified").metadata, Default::default());
    assert_eq!(node("ordinary").action, Action::Create);
    assert_eq!(node("ordinary").metadata, Default::default());
    assert!(!format!("{raw:?}").contains("TOP_SECRET"));
}

#[test]
fn invalid_path_steps_do_not_become_truncated_replacement_paths() {
    use crate::model::AttributePathStep as Step;
    let metadata = parse(
        &json!({"replace_paths":[[],["valid",4],["bad",true],["negative",-1],"invalid",["fraction",1.5]]}),
    );
    assert_eq!(
        metadata.replace_paths,
        Some(vec![
            vec![],
            vec![Step::Attribute("valid".into()), Step::Index(4)]
        ])
    );
    assert_eq!(
        parse(&json!({"replace_paths":[]})).replace_paths,
        Some(vec![])
    );
    assert_eq!(parse(&json!({"replace_paths":false})).replace_paths, None);
}

#[test]
fn import_metadata_is_value_free_and_independent_of_planned_action() {
    for importing in [
        json!({}),
        json!({"id":"TOP_SECRET"}),
        json!({"identity":{"secret":"TOP_SECRET"}}),
        json!({"unknown":true}),
    ] {
        let input = json!({"format_version":"1.2","resource_changes":[{"address":"terraform_data.main","type":"terraform_data","change":{"actions":["update"],"importing":importing,"before":{"secret":"TOP_SECRET"}}}]});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        let node = &raw.nodes[0];
        assert_eq!(node.action, crate::model::Action::Update);
        let metadata = node.metadata.import.unwrap();
        assert_eq!(metadata.has_id, importing.get("id").is_some());
        assert_eq!(metadata.has_identity, importing.get("identity").is_some());
        assert_eq!(metadata.unknown, importing["unknown"] == true);
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
    }
    for importing in [Value::Null, json!(false), json!("TOP_SECRET")] {
        assert!(parse(&json!({"importing":importing})).import.is_none());
    }
}

#[test]
fn state_removal_matches_only_supported_complete_action_sequences() {
    for (actions, expected) in [
        (json!(["forget"]), Some(StateRemoval::Forget)),
        (
            json!(["create", "forget"]),
            Some(StateRemoval::CreateThenForget),
        ),
        (
            json!(["forget", "create"]),
            Some(StateRemoval::ForgetThenCreate),
        ),
        (json!(["delete"]), None),
        (json!(["delete", "create"]), None),
        (json!(["forget", "future"]), None),
        (json!(["forget", null]), None),
    ] {
        assert_eq!(parse(&json!({"actions":actions})).state_removal, expected);
    }
}

#[test]
fn lowering_keeps_lifecycle_metadata_on_the_relationship() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    input["resource_changes"][2]["change"] =
        json!({"actions":["forget"],"importing":{"id":"TOP_SECRET"}});
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    let change = graph.edges.iter().find_map(|e| e.change.as_ref()).unwrap();
    assert_eq!(change.metadata.state_removal, Some(StateRemoval::Forget));
    assert!(change.metadata.import.unwrap().has_id);
    assert!(!format!("{graph:?}").contains("TOP_SECRET"));
}
