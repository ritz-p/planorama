use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/messaging_policies.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_sns_topic_policy" | "aws_sns_topic_data_protection_policy", "arn") => {
            Some("Policy topic")
        }
        (
            "aws_sqs_queue_policy"
            | "aws_sqs_queue_redrive_policy"
            | "aws_sqs_queue_redrive_allow_policy",
            "queue_url",
        ) => Some("Policy queue"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::service_evidence::annotate_scalar(
        nodes,
        refs,
        evidence,
        label,
        |_, path, source, field| match (path, source) {
            ("arn", "aws_sns_topic") => field == "arn",
            ("queue_url", "aws_sqs_queue") => matches!(field, "url" | "id"),
            _ => false,
        },
    );
}
