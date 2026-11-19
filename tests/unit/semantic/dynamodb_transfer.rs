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
        serde_json::from_str(include_str!("../../fixtures/dynamodb-transfer-plan.json")).unwrap();
    for (address, path) in [
        ("aws_dynamodb_table_export.sample1", "table_arn"),
        ("aws_dynamodb_table_export.sample1", "s3_bucket"),
        ("aws_dynamodb_table_export.sample1", "s3_sse_kms_key_id"),
        ("aws_dynamodb_table.sample4", "restore_source_name"),
        (
            "aws_dynamodb_table.sample5",
            "import_table.s3_bucket_source.bucket",
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
    changed["resource_changes"][1]["change"]["after"]["s3_sse_algorithm"] = json!("AES256");
    assert!(!verified(
        &changed,
        "aws_dynamodb_table_export.sample1",
        "s3_sse_kms_key_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["change"]["after"]["import_table"][0]["s3_bucket_source"][0]["bucket_owner"] =
        json!("987654321098");
    assert!(!verified(
        &changed,
        "aws_dynamodb_table.sample5",
        "import_table.s3_bucket_source.bucket"
    ));
}
