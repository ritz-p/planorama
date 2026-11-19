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
        serde_json::from_str(include_str!("../../fixtures/fis-configuration-plan.json")).unwrap();
    for (address, path) in [
        ("aws_fis_experiment_template.sample0", "role_arn"),
        (
            "aws_fis_experiment_template.sample0",
            "target.resource_arns",
        ),
        (
            "aws_fis_experiment_template.sample0",
            "stop_condition.value",
        ),
        (
            "aws_fis_experiment_template.sample0",
            "log_configuration.cloudwatch_logs_configuration.log_group_arn",
        ),
        (
            "aws_fis_experiment_template.sample0",
            "log_configuration.s3_configuration.bucket_name",
        ),
        (
            "aws_fis_experiment_template.sample0",
            "experiment_report_configuration.data_sources.cloudwatch_dashboard.dashboard_arn",
        ),
        (
            "aws_fis_experiment_template.sample0",
            "experiment_report_configuration.outputs.s3_configuration.bucket_name",
        ),
        (
            "aws_fis_target_account_configuration.sample7",
            "experiment_template_id",
        ),
        ("aws_fis_target_account_configuration.sample7", "account_id"),
        ("aws_fis_target_account_configuration.sample7", "role_arn"),
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
        "aws_fis_experiment_template.sample0",
        "role_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_fis_experiment_template.sample0",
        "role_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["arn"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_fis_experiment_template.sample0",
        "role_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["target"][0]["resource_type"] =
        json!("aws:ec2:volume");
    assert!(!verified(
        &changed,
        "aws_fis_experiment_template.sample0",
        "target.resource_arns"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["stop_condition"][0]["source"] =
        json!("none");
    assert!(!verified(
        &changed,
        "aws_fis_experiment_template.sample0",
        "stop_condition.value"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_fis_experiment_template.sample0",
        "role_arn"
    ));
}
