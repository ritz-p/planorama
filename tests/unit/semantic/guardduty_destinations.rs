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
    let fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/guardduty-destinations-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_guardduty_malware_protection_plan.sample0", "role"),
        (
            "aws_guardduty_malware_protection_plan.sample0",
            "protected_resource.s3_bucket.bucket_name",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "detector_id",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "destination_arn",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "kms_key_arn",
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
        "aws_guardduty_malware_protection_plan.sample0",
        "role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_guardduty_malware_protection_plan.sample0",
        "role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["arn"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_guardduty_malware_protection_plan.sample0",
        "role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["destination_type"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_guardduty_publishing_destination.sample3",
        "destination_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_guardduty_malware_protection_plan.sample0",
        "role"
    ));
}
