use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_config_configuration_recorder", "role_arn") => {
            Rule::new("Recorder role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_config_configuration_recorder_status", "name") => Rule::new(
            "Configuration recorder",
            &[("aws_config_configuration_recorder", &["name"])],
            false,
            &[],
        ),
        (
            "aws_config_conformance_pack" | "aws_config_organization_conformance_pack",
            "delivery_s3_bucket",
        ) => Rule::new(
            "Conformance delivery bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_config_delivery_channel", "s3_bucket_name") => Rule::new(
            "Configuration delivery bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_config_delivery_channel", "sns_topic_arn") => Rule::new(
            "Configuration notification topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[],
        ),
        ("aws_config_delivery_channel", "s3_kms_key_arn") => Rule::new(
            "Configuration delivery key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_config_configuration_recorder"
            | "aws_config_configuration_recorder_status"
            | "aws_config_conformance_pack"
            | "aws_config_organization_conformance_pack"
            | "aws_config_delivery_channel"
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
fn qualifies(_e: &Evidence<'_>, _r: &AttributeReference, _source: usize, _path: &str) -> bool {
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/config_delivery.rs"]
mod tests;
