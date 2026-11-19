use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_comprehend_document_classifier" | "aws_comprehend_entity_recognizer",
            "data_access_role_arn",
        ) => Rule::new(
            "Training data role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_comprehend_document_classifier" | "aws_comprehend_entity_recognizer",
            "model_kms_key_id",
        ) => Rule::new(
            "Model encryption key",
            &[("aws_kms_key", &["id", "key_id", "arn"])],
            false,
            &[],
        ),
        (
            "aws_comprehend_document_classifier" | "aws_comprehend_entity_recognizer",
            "volume_kms_key_id",
        ) => Rule::new(
            "Training volume key",
            &[("aws_kms_key", &["id", "key_id", "arn"])],
            false,
            &[],
        ),
        (
            "aws_comprehend_document_classifier" | "aws_comprehend_entity_recognizer",
            "vpc_config.subnets",
        ) => Rule::new("Training subnets", &[("aws_subnet", &["id"])], true, &[]),
        (
            "aws_comprehend_document_classifier" | "aws_comprehend_entity_recognizer",
            "vpc_config.security_group_ids",
        ) => Rule::new(
            "Training security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_rekognition_stream_processor", "role_arn") => Rule::new(
            "Stream processing role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "input.kinesis_video_stream.arn") => Rule::new(
            "Video stream",
            &[("aws_kinesis_video_stream", &["arn"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "output.kinesis_data_stream.arn") => Rule::new(
            "Result stream",
            &[("aws_kinesis_stream", &["arn"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "output.s3_destination.bucket") => Rule::new(
            "Results bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "notification_channel.sns_topic_arn") => Rule::new(
            "Notification topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "settings.face_search.collection_id") => Rule::new(
            "Face search collection",
            &[("aws_rekognition_collection", &["collection_id", "id"])],
            false,
            &[],
        ),
        ("aws_rekognition_stream_processor", "kms_key_id") => Rule::new(
            "Label detection encryption key",
            &[("aws_kms_key", &["id", "key_id", "arn"])],
            false,
            &[],
        ),
        ("aws_transcribe_language_model", "input_data_config.data_access_role_arn") => Rule::new(
            "Training data role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_comprehend_document_classifier"
            | "aws_comprehend_entity_recognizer"
            | "aws_rekognition_stream_processor"
            | "aws_transcribe_language_model"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, _path: &str) -> bool {
    let configured = |path| {
        e.value(r.target, path)
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.len() == 1)
    };
    let absent = |path| {
        e.value(r.target, path)
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
    };
    let face = || configured("settings.0.face_search") && absent("settings.0.connected_home");
    let labels = || configured("settings.0.connected_home") && absent("settings.0.face_search");
    match r.attribute.as_str() {
        "vpc_config.subnets" | "vpc_config.security_group_ids" => {
            configured_links::network_context(
                e,
                r.target,
                &[
                    ("vpc_config.subnets", "aws_subnet"),
                    ("vpc_config.security_group_ids", "aws_security_group"),
                ],
            )
            .is_some()
        }
        "output.kinesis_data_stream.arn" => face() && absent("output.0.s3_destination"),
        "settings.face_search.collection_id" => face(),
        "output.s3_destination.bucket" => labels() && absent("output.0.kinesis_data_stream"),
        "notification_channel.sns_topic_arn" | "kms_key_id" => labels(),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ai_service_configuration.rs"]
mod tests;
