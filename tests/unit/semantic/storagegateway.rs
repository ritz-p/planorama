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
        "../../fixtures/storagegateway-config-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_storagegateway_gateway.sample0",
            "cloudwatch_log_group_arn",
        ),
        ("aws_storagegateway_nfs_file_share.sample1", "gateway_arn"),
        ("aws_storagegateway_nfs_file_share.sample1", "location_arn"),
        ("aws_storagegateway_nfs_file_share.sample1", "kms_key_arn"),
        ("aws_storagegateway_cache.sample5", "gateway_arn"),
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
    changed["resource_changes"][1]["change"]["after"]["location_arn"] =
        json!("arn:aws:s3:::fixture-gateway/prefix");
    assert!(!verified(
        &changed,
        "aws_storagegateway_nfs_file_share.sample1",
        "location_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["kms_encrypted"] = json!(false);
    assert!(!verified(
        &changed,
        "aws_storagegateway_nfs_file_share.sample1",
        "kms_key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_storagegateway_nfs_file_share.sample1",
        "gateway_arn"
    ));
}
