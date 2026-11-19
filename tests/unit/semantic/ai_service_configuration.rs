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
        "../../fixtures/ai-service-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_comprehend_document_classifier.sample0",
            "data_access_role_arn",
        ),
        (
            "aws_comprehend_document_classifier.sample0",
            "model_kms_key_id",
        ),
        (
            "aws_comprehend_document_classifier.sample0",
            "volume_kms_key_id",
        ),
        (
            "aws_comprehend_document_classifier.sample0",
            "vpc_config.subnets",
        ),
        (
            "aws_comprehend_document_classifier.sample0",
            "vpc_config.security_group_ids",
        ),
        (
            "aws_comprehend_entity_recognizer.sample1",
            "data_access_role_arn",
        ),
        (
            "aws_comprehend_entity_recognizer.sample1",
            "model_kms_key_id",
        ),
        (
            "aws_comprehend_entity_recognizer.sample1",
            "volume_kms_key_id",
        ),
        (
            "aws_comprehend_entity_recognizer.sample1",
            "vpc_config.subnets",
        ),
        (
            "aws_comprehend_entity_recognizer.sample1",
            "vpc_config.security_group_ids",
        ),
        ("aws_rekognition_stream_processor.sample2", "role_arn"),
        (
            "aws_rekognition_stream_processor.sample2",
            "input.kinesis_video_stream.arn",
        ),
        (
            "aws_rekognition_stream_processor.sample2",
            "output.kinesis_data_stream.arn",
        ),
        (
            "aws_rekognition_stream_processor.sample2",
            "settings.face_search.collection_id",
        ),
        (
            "aws_rekognition_stream_processor.sample3",
            "output.s3_destination.bucket",
        ),
        (
            "aws_rekognition_stream_processor.sample3",
            "notification_channel.sns_topic_arn",
        ),
        ("aws_rekognition_stream_processor.sample3", "kms_key_id"),
        (
            "aws_transcribe_language_model.sample4",
            "input_data_config.data_access_role_arn",
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
    changed["resource_changes"][0]["change"]["after"]["vpc_config"][0]["security_group_ids"] =
        json!(["sg-fixture", "sg-unproven"]);
    assert!(!verified(
        &changed,
        "aws_comprehend_document_classifier.sample0",
        "vpc_config.subnets"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][2]["change"]["after"]["settings"][0]["connected_home"] =
        json!([{"labels":["PERSON"]}]);
    assert!(!verified(
        &changed,
        "aws_rekognition_stream_processor.sample2",
        "output.kinesis_data_stream.arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["output"][0]["kinesis_data_stream"] =
        json!([{"arn":"other"}]);
    assert!(!verified(
        &changed,
        "aws_rekognition_stream_processor.sample3",
        "output.s3_destination.bucket"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_comprehend_document_classifier.sample0",
        "data_access_role_arn"
    ));
}
