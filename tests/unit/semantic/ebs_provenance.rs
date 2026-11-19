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
        serde_json::from_str(include_str!("../../fixtures/ebs-provenance-plan.json")).unwrap();
    for (address, path) in [
        ("aws_ebs_snapshot_copy.sample0", "source_snapshot_id"),
        ("aws_ebs_snapshot_copy.sample0", "kms_key_id"),
        ("aws_ebs_snapshot.sample1", "volume_id"),
        ("aws_ebs_volume_copy.sample4", "source_volume_id"),
        (
            "aws_ebs_snapshot_import.sample5",
            "disk_container.user_bucket.s3_bucket",
        ),
        ("aws_ebs_fast_snapshot_restore.sample7", "snapshot_id"),
        ("aws_ebs_default_kms_key.sample8", "key_arn"),
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
    changed["resource_changes"][0]["change"]["after"]["source_region"] = json!("us-west-1");
    assert!(!verified(
        &changed,
        "aws_ebs_snapshot_copy.sample0",
        "source_snapshot_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["encrypted"] = json!(false);
    assert!(!verified(
        &changed,
        "aws_ebs_snapshot_copy.sample0",
        "kms_key_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ebs_snapshot_copy.sample0",
        "source_snapshot_id"
    ));
}
