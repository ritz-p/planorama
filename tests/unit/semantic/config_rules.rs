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
        serde_json::from_str(include_str!("../../fixtures/config-rules-plan.json")).unwrap();
    for (address, path) in [
        ("aws_config_config_rule.sample0", "source.source_identifier"),
        (
            "aws_config_organization_custom_rule.sample2",
            "lambda_function_arn",
        ),
        (
            "aws_config_remediation_configuration.sample3",
            "config_rule_name",
        ),
        ("aws_config_remediation_configuration.sample3", "target_id"),
    ] {
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
        "aws_config_config_rule.sample0",
        "source.source_identifier"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_config_config_rule.sample0",
        "source.source_identifier"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["arn"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_config_config_rule.sample0",
        "source.source_identifier"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["source"][0]["owner"] = json!("AWS");
    assert!(!verified(
        &changed,
        "aws_config_config_rule.sample0",
        "source.source_identifier"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_config_config_rule.sample0",
        "source.source_identifier"
    ));
}
