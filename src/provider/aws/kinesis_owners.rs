use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/kinesis_owners.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_kinesis_stream_consumer", "stream_arn") => Some("Consumer stream"),
        ("aws_kinesis_resource_policy", "resource_arn") => Some("Kinesis policy resource"),
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
        |kind, path, source, field| {
            matches!(
                (kind, path, source, field),
                (
                    "aws_kinesis_stream_consumer",
                    "stream_arn",
                    "aws_kinesis_stream",
                    "arn"
                ) | (
                    "aws_kinesis_resource_policy",
                    "resource_arn",
                    "aws_kinesis_stream" | "aws_kinesis_stream_consumer",
                    "arn"
                )
            )
        },
    );
}
