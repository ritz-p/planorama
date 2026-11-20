use super::Rule;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudcontrolapi_resource" | "aws_cloudformation_stack" | "aws_cloudformation_type"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudcontrolapi_resource", "role_arn") => Rule::new(
            "Cloud Control operations role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_cloudformation_stack", "iam_role_arn") => Rule::new(
            "Stack execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudformation_stack", "notification_arns") => Rule::new(
            "Stack notification topics",
            &[("aws_sns_topic", &["arn"])],
            true,
            &[],
        ),
        ("aws_cloudformation_type", "execution_role_arn") => Rule::new(
            "Registry extension execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudformation_type", "logging_config.log_role_arn") => Rule::new(
            "Registry extension logging role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudformation_type", "logging_config.log_group_name") => Rule::new(
            "Registry extension log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
