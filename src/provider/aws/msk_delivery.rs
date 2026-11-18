use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_msk_topic" | "aws_msk_cluster_policy" | "aws_msk_channel"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_msk_topic", "cluster_arn") => Rule::new(
            "MSK cluster",
            &[
                ("aws_msk_cluster", &["arn"]),
                ("aws_msk_serverless_cluster", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_msk_cluster_policy", "cluster_arn") => Rule::new(
            "MSK cluster",
            &[
                ("aws_msk_cluster", &["arn"]),
                ("aws_msk_serverless_cluster", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_msk_channel", "cluster_arn") => Rule::new(
            "Channel cluster",
            &[("aws_msk_cluster", &["arn"])],
            false,
            &[],
        ),
        ("aws_msk_channel", "topic_configuration.topic_arn") => Rule::new(
            "Channel topic",
            &[("aws_msk_topic", &["arn"])],
            false,
            &["topic_configuration"],
        ),
        ("aws_msk_channel", "s3_destination.service_execution_role_arn") => Rule::new(
            "Delivery execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["s3_destination"],
        ),
        ("aws_msk_channel", "s3_destination.dead_letter_queue_s3.bucket_arn") => Rule::new(
            "Rejected records bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &["s3_destination", "s3_destination.dead_letter_queue_s3"],
        ),
        ("aws_msk_channel", "iceberg_destination.service_execution_role_arn") => Rule::new(
            "Delivery execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["iceberg_destination"],
        ),
        ("aws_msk_channel", "iceberg_destination.dead_letter_queue_s3.bucket_arn") => Rule::new(
            "Rejected records bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &[
                "iceberg_destination",
                "iceberg_destination.dead_letter_queue_s3",
            ],
        ),
        ("aws_msk_channel", "s3_destination.storage.bucket_arn") => Rule::new(
            "Channel storage bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &["s3_destination", "s3_destination.storage"],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, _path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_msk_channel", "topic_configuration.topic_arn") => e.same_endpoint(
            r.target,
            "cluster_arn",
            s,
            "cluster_arn",
            "aws_msk_cluster",
            "arn",
        ),
        ("aws_msk_channel", p) if p.starts_with("s3_destination.") => {
            empty(e, r.target, "iceberg_destination")
        }
        ("aws_msk_channel", p) if p.starts_with("iceberg_destination.") => {
            empty(e, r.target, "s3_destination")
        }
        _ => true,
    }
}
fn empty(e: &Evidence<'_>, i: usize, p: &str) -> bool {
    e.value(i, p)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/msk_delivery.rs"]
mod tests;
