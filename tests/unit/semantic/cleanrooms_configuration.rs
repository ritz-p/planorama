use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
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
    let fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/cleanrooms-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_cleanrooms_configured_table.sample1",
            "table_reference.database_name",
        ),
        (
            "aws_cleanrooms_configured_table.sample1",
            "table_reference.table_name",
        ),
        ("aws_cleanrooms_membership.sample2", "collaboration_id"),
        (
            "aws_cleanrooms_membership.sample2",
            "default_result_configuration.role_arn",
        ),
        (
            "aws_cleanrooms_membership.sample2",
            "default_result_configuration.output_configuration.s3.bucket",
        ),
    ] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][4]["change"]["after"]["catalog_id"] = json!("234567890123");
    assert!(!verified(
        &changed,
        "aws_cleanrooms_configured_table.sample1",
        "table_reference.table_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][2]["change"]["after"]["collaboration_creator_account_id"] =
        json!("234567890123");
    assert!(!verified(
        &changed,
        "aws_cleanrooms_membership.sample2",
        "collaboration_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_cleanrooms_configured_table.sample1",
        "table_reference.database_name"
    ));
}
