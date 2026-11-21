use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_elastictranscoder_pipeline", "input_bucket") => Rule::new(
            "Transcoder input bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "output_bucket") => Rule::new(
            "Transcoder output bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "role") => Rule::new(
            "Transcoder service role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "aws_kms_key_arn") => Rule::new(
            "Transcoder encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, |_, _, _, _| true)
}
