use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    node.references.iter().any(|reference| {
        reference.attribute == path
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
            && super::label(&node.resource_type, path) == Some(reference.label)
    })
}

#[test]
fn configured_links_require_current_typed_evidence() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/access-analyzer-plan.json")).unwrap();
    for (address, path) in [("aws_accessanalyzer_archive_rule.sample0", "analyzer_name")] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(true);
        assert!(
            !verified(&masked, address, path),
            "{address}/{path} sensitive"
        );
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(false);
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(
        &changed,
        "aws_accessanalyzer_archive_rule.sample0",
        "analyzer_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_accessanalyzer_archive_rule.sample0",
        "analyzer_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["analyzer_name"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_accessanalyzer_archive_rule.sample0",
        "analyzer_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_accessanalyzer_archive_rule.sample0",
        "analyzer_name"
    ));
}

#[test]
fn only_a_current_managed_analyzer_in_the_same_region_can_own_the_rule() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/access-analyzer-plan.json")).unwrap();
    let owner = |input: &Value| {
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_accessanalyzer_archive_rule")
            .unwrap()
            .member_owner
            .is_some()
    };
    assert!(owner(&fixture));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["region"] = json!("eu-west-1");
    changed["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
        json!({"constant_value":"eu-west-1"});
    assert!(!owner(&changed));
    let mut literal = fixture.clone();
    literal["configuration"]["root_module"]["resources"][0]["expressions"]["analyzer_name"] =
        json!({"constant_value": "accessanalyzer_analyzer-fixture"});
    assert!(!owner(&literal));
    let mut lookup = fixture;
    lookup["resource_changes"][1]["mode"] = json!("data");
    assert!(!owner(&lookup));
}
