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
        serde_json::from_str(include_str!("../../fixtures/security-lake-plan.json")).unwrap();
    for (address, path) in [
        ("aws_securitylake_aws_log_source.sample0", "source.accounts"),
        (
            "aws_securitylake_custom_log_source.sample2",
            "configuration.crawler_configuration.role_arn",
        ),
        (
            "aws_securitylake_data_lake.sample4",
            "configuration.encryption_configuration",
        ),
        (
            "aws_securitylake_data_lake.sample4",
            "configuration.replication_configuration.role_arn",
        ),
        (
            "aws_securitylake_data_lake.sample4",
            "meta_store_manager_role_arn",
        ),
        (
            "aws_securitylake_subscriber.sample6",
            "source.custom_log_source_resource.source_name",
        ),
        (
            "aws_securitylake_subscriber_notification.sample7",
            "subscriber_id",
        ),
        (
            "aws_securitylake_subscriber_notification.sample7",
            "configuration.https_notification_configuration.target_role_arn",
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
        "aws_securitylake_aws_log_source.sample0",
        "source.accounts"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_securitylake_aws_log_source.sample0",
        "source.accounts"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["id"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_securitylake_aws_log_source.sample0",
        "source.accounts"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_securitylake_aws_log_source.sample0",
        "source.accounts"
    ));
}
