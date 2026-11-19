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
        "../../fixtures/transfer-workflow-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_transfer_workflow.sample0",
            "steps.custom_step_details.target",
        ),
        (
            "aws_transfer_workflow.sample0",
            "steps.copy_step_details.destination_file_location.efs_file_location.file_system_id",
        ),
        (
            "aws_transfer_workflow.sample0",
            "steps.copy_step_details.destination_file_location.s3_file_location.bucket",
        ),
        (
            "aws_transfer_workflow.sample0",
            "steps.decrypt_step_details.destination_file_location.efs_file_location.file_system_id",
        ),
        (
            "aws_transfer_workflow.sample0",
            "steps.decrypt_step_details.destination_file_location.s3_file_location.bucket",
        ),
        (
            "aws_transfer_workflow.sample0",
            "on_exception_steps.custom_step_details.target",
        ),
        (
            "aws_transfer_workflow.sample0",
            "on_exception_steps.copy_step_details.destination_file_location.efs_file_location.file_system_id",
        ),
        (
            "aws_transfer_workflow.sample0",
            "on_exception_steps.copy_step_details.destination_file_location.s3_file_location.bucket",
        ),
        (
            "aws_transfer_workflow.sample0",
            "on_exception_steps.decrypt_step_details.destination_file_location.efs_file_location.file_system_id",
        ),
        (
            "aws_transfer_workflow.sample0",
            "on_exception_steps.decrypt_step_details.destination_file_location.s3_file_location.bucket",
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
    changed["resource_changes"][0]["change"]["after"]["on_exception_steps"][0]["type"] =
        json!("DELETE");
    assert!(!verified(
        &changed,
        "aws_transfer_workflow.sample0",
        "on_exception_steps.custom_step_details.target"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["steps"][1]["copy_step_details"][0]["destination_file_location"]
        [0]["s3_file_location"] = json!([{"bucket":"fixture-bucket"}]);
    assert!(!verified(
        &changed,
        "aws_transfer_workflow.sample0",
        "steps.copy_step_details.destination_file_location.efs_file_location.file_system_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_transfer_workflow.sample0",
        "steps.custom_step_details.target"
    ));
}
