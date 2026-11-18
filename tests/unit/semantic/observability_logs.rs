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
        serde_json::from_str(include_str!("../../fixtures/observability-logs-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_observabilityadmin_s3_table_integration.sample1",
            "encryption.kms_key_arn",
        ),
        (
            "aws_cloudwatch_log_s3_table_integration_source.sample2",
            "integration_arn",
        ),
        (
            "aws_observabilityadmin_centralization_rule_for_organization.sample5",
            "rule.destination.destination_logs_configuration.logs_encryption_configuration.kms_key_arn",
        ),
        (
            "aws_observabilityadmin_centralization_rule_for_organization.sample5",
            "rule.destination.destination_logs_configuration.tag_propagation_configuration.destination_role_arn",
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
    changed["resource_changes"][1]["change"]["after"]["encryption"][0]["sse_algorithm"] =
        json!("AES256");
    assert!(!verified(
        &changed,
        "aws_observabilityadmin_s3_table_integration.sample1",
        "encryption.kms_key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["change"]["after"]["rule"][0]["destination"][0]["region"] =
        json!("us-east-1");
    assert!(!verified(
        &changed,
        "aws_observabilityadmin_centralization_rule_for_organization.sample5",
        "rule.destination.destination_logs_configuration.logs_encryption_configuration.kms_key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["change"]["after"]["rule"][0]["destination"][0]["account"] =
        json!("987654321098");
    assert!(!verified(
        &changed,
        "aws_observabilityadmin_centralization_rule_for_organization.sample5",
        "rule.destination.destination_logs_configuration.tag_propagation_configuration.destination_role_arn"
    ));
}
