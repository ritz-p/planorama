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
        serde_json::from_str(include_str!("../../fixtures/flink-configuration-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_kinesisanalyticsv2_application.sample0",
            "application_configuration.vpc_configuration.subnet_ids",
        ),
        (
            "aws_kinesisanalyticsv2_application.sample0",
            "application_configuration.vpc_configuration.security_group_ids",
        ),
        (
            "aws_kinesisanalyticsv2_application.sample0",
            "service_execution_role",
        ),
        (
            "aws_kinesisanalyticsv2_application.sample0",
            "application_configuration.application_code_configuration.code_content.s3_content_location.bucket_arn",
        ),
        (
            "aws_kinesisanalyticsv2_application_snapshot.sample6",
            "application_name",
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
    changed["resource_changes"][0]["change"]["after"]["application_configuration"][0]["vpc_configuration"]
        [0]["security_group_ids"] = json!(["sg-fixture", "sg-unproven"]);
    assert!(!verified(
        &changed,
        "aws_kinesisanalyticsv2_application.sample0",
        "application_configuration.vpc_configuration.subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["runtime_environment"] = json!("SQL-1_0");
    assert!(!verified(
        &changed,
        "aws_kinesisanalyticsv2_application_snapshot.sample6",
        "application_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_kinesisanalyticsv2_application.sample0",
        "application_configuration.vpc_configuration.subnet_ids"
    ));
}
