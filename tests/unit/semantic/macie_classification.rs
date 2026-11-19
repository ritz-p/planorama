use crate::semantic;
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
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
        "../../fixtures/macie-classification-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_macie2_classification_export_configuration.sample0",
            "s3_destination.bucket_name",
        ),
        (
            "aws_macie2_classification_export_configuration.sample0",
            "s3_destination.kms_key_arn",
        ),
        (
            "aws_macie2_classification_job.sample3",
            "custom_data_identifier_ids",
        ),
        (
            "aws_macie2_classification_job.sample3",
            "s3_job_definition.bucket_definitions.buckets",
        ),
        (
            "aws_macie2_classification_job.sample3",
            "s3_job_definition.bucket_definitions.account_id",
        ),
        (
            "aws_macie2_invitation_accepter.sample6",
            "administrator_account_id",
        ),
        ("aws_macie2_member.sample7", "account_id"),
        (
            "aws_macie2_organization_admin_account.sample8",
            "admin_account_id",
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
        "aws_macie2_classification_export_configuration.sample0",
        "s3_destination.bucket_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_macie2_classification_export_configuration.sample0",
        "s3_destination.bucket_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["bucket"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_macie2_classification_export_configuration.sample0",
        "s3_destination.bucket_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_macie2_classification_export_configuration.sample0",
        "s3_destination.bucket_name"
    ));
}
