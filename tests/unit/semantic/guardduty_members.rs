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
        serde_json::from_str(include_str!("../../fixtures/guardduty-members-plan.json")).unwrap();
    for (address, path) in [
        ("aws_guardduty_detector_feature.sample0", "detector_id"),
        ("aws_guardduty_filter.sample2", "detector_id"),
        ("aws_guardduty_invite_accepter.sample3", "detector_id"),
        ("aws_guardduty_ipset.sample4", "detector_id"),
        ("aws_guardduty_member.sample5", "detector_id"),
        (
            "aws_guardduty_member_detector_feature.sample6",
            "detector_id",
        ),
        (
            "aws_guardduty_organization_configuration.sample7",
            "detector_id",
        ),
        (
            "aws_guardduty_organization_configuration_feature.sample8",
            "detector_id",
        ),
        ("aws_guardduty_threatintelset.sample9", "detector_id"),
        ("aws_guardduty_member.sample5", "account_id"),
        (
            "aws_guardduty_member_detector_feature.sample6",
            "account_id",
        ),
        (
            "aws_guardduty_organization_admin_account.sample11",
            "admin_account_id",
        ),
        ("aws_guardduty_invite_accepter.sample3", "master_account_id"),
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
        "aws_guardduty_detector_feature.sample0",
        "detector_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_guardduty_detector_feature.sample0",
        "detector_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["id"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_guardduty_detector_feature.sample0",
        "detector_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_guardduty_detector_feature.sample0",
        "detector_id"
    ));
}
