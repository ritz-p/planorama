use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_transfer_workflow", "steps.custom_step_details.target") => Rule::new(
            "Workflow Lambda function",
            &[
                ("aws_lambda_function", &["arn", "qualified_arn"]),
                ("aws_lambda_alias", &["arn"]),
            ],
            false,
            &["steps"],
        ),
        (
            "aws_transfer_workflow",
            "steps.copy_step_details.destination_file_location.efs_file_location.file_system_id",
        ) => Rule::new(
            "Copy destination EFS",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["steps"],
        ),
        (
            "aws_transfer_workflow",
            "steps.copy_step_details.destination_file_location.s3_file_location.bucket",
        ) => Rule::new(
            "Copy destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["steps"],
        ),
        (
            "aws_transfer_workflow",
            "steps.decrypt_step_details.destination_file_location.efs_file_location.file_system_id",
        ) => Rule::new(
            "Decrypt destination EFS",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["steps"],
        ),
        (
            "aws_transfer_workflow",
            "steps.decrypt_step_details.destination_file_location.s3_file_location.bucket",
        ) => Rule::new(
            "Decrypt destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["steps"],
        ),
        ("aws_transfer_workflow", "on_exception_steps.custom_step_details.target") => Rule::new(
            "Workflow Lambda function",
            &[
                ("aws_lambda_function", &["arn", "qualified_arn"]),
                ("aws_lambda_alias", &["arn"]),
            ],
            false,
            &["on_exception_steps"],
        ),
        (
            "aws_transfer_workflow",
            "on_exception_steps.copy_step_details.destination_file_location.efs_file_location.file_system_id",
        ) => Rule::new(
            "Copy destination EFS",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["on_exception_steps"],
        ),
        (
            "aws_transfer_workflow",
            "on_exception_steps.copy_step_details.destination_file_location.s3_file_location.bucket",
        ) => Rule::new(
            "Copy destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["on_exception_steps"],
        ),
        (
            "aws_transfer_workflow",
            "on_exception_steps.decrypt_step_details.destination_file_location.efs_file_location.file_system_id",
        ) => Rule::new(
            "Decrypt destination EFS",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["on_exception_steps"],
        ),
        (
            "aws_transfer_workflow",
            "on_exception_steps.decrypt_step_details.destination_file_location.s3_file_location.bucket",
        ) => Rule::new(
            "Decrypt destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["on_exception_steps"],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_transfer_workflow")
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    let mut parts = path.split('.');
    let (Some(section), Some(index), Some(step)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let expected = match step {
        "custom_step_details" => "CUSTOM",
        "copy_step_details" => "COPY",
        "decrypt_step_details" => "DECRYPT",
        _ => return false,
    };
    if e.text(r.target, &format!("{section}.{index}.type")) != Some(expected) {
        return false;
    }
    if expected == "CUSTOM" {
        return true;
    }
    let other = if path.ends_with(".bucket") {
        "efs_file_location"
    } else {
        "s3_file_location"
    };
    e.value(
        r.target,
        &format!("{section}.{index}.{step}.0.destination_file_location.0.{other}"),
    )
    .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transfer_workflow_configuration.rs"]
mod tests;
