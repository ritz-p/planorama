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
        "../../fixtures/dataexchange-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_dataexchange_revision.sample2", "data_set_id"),
        ("aws_dataexchange_revision_assets.sample3", "data_set_id"),
        ("aws_dataexchange_revision_assets.sample4", "data_set_id"),
        (
            "aws_dataexchange_revision_assets.sample3",
            "asset.create_s3_data_access_from_s3_bucket.asset_source.bucket",
        ),
        (
            "aws_dataexchange_revision_assets.sample3",
            "asset.create_s3_data_access_from_s3_bucket.asset_source.kms_keys_to_grant.kms_key_arn",
        ),
        (
            "aws_dataexchange_revision_assets.sample4",
            "asset.import_assets_from_s3.asset_source.bucket",
        ),
        (
            "aws_dataexchange_event_action.sample5",
            "event.revision_published.data_set_id",
        ),
        (
            "aws_dataexchange_event_action.sample5",
            "action.export_revision_to_s3.revision_destination.bucket",
        ),
        (
            "aws_dataexchange_event_action.sample5",
            "action.export_revision_to_s3.encryption.kms_key_arn",
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
    changed["resource_changes"][0]["change"]["after"]["asset_type"] = json!("S3_SNAPSHOT");
    assert!(!verified(
        &changed,
        "aws_dataexchange_revision_assets.sample3",
        "asset.create_s3_data_access_from_s3_bucket.asset_source.bucket"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["change"]["after"]["action"][0]["export_revision_to_s3"][0]["encryption"]
        [0]["type"] = json!("aws:s3");
    assert!(!verified(
        &changed,
        "aws_dataexchange_event_action.sample5",
        "action.export_revision_to_s3.encryption.kms_key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_dataexchange_revision.sample2",
        "data_set_id"
    ));
}
