use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_kinesis_firehose_delivery_stream")
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_kinesis_firehose_delivery_stream", "msk_source_configuration.msk_cluster_arn") => {
            Rule::new(
                "MSK source",
                &[
                    ("aws_msk_cluster", &["arn"]),
                    ("aws_msk_serverless_cluster", &["arn"]),
                ],
                false,
                &[],
            )
        }
        (
            "aws_kinesis_firehose_delivery_stream",
            "msk_source_configuration.authentication_configuration.role_arn",
        ) => Rule::new("MSK source role", &[("aws_iam_role", &["arn"])], false, &[]),
        ("aws_kinesis_firehose_delivery_stream", "kinesis_source_configuration.role_arn") => {
            Rule::new(
                "Kinesis source role",
                &[("aws_iam_role", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_kinesis_firehose_delivery_stream", "extended_s3_configuration.role_arn") => {
            Rule::new(
                "S3 delivery role",
                &[("aws_iam_role", &["arn"])],
                false,
                &[],
            )
        }
        (
            "aws_kinesis_firehose_delivery_stream",
            "extended_s3_configuration.s3_backup_configuration.role_arn",
        ) => Rule::new("S3 backup role", &[("aws_iam_role", &["arn"])], false, &[]),
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _s: usize, _path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (_, p) => {
            let (v, m) = e.values[r.target];
            super::firehose::configuration_verified(p, v, m)
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/firehose_sources.rs"]
mod tests;
