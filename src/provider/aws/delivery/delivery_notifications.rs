use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_codecommit_trigger" | "aws_codestarnotifications_notification_rule"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codecommit_trigger", "repository_name") => Rule::new(
            "Trigger repository",
            &[("aws_codecommit_repository", &["repository_name"])],
            false,
            &[],
        ),
        ("aws_codecommit_trigger", "trigger.destination_arn") => Rule::new(
            "Repository trigger destination",
            &[
                ("aws_sns_topic", &["arn"]),
                ("aws_lambda_function", &["arn"]),
            ],
            false,
            &["trigger"],
        ),
        ("aws_codestarnotifications_notification_rule", "resource") => Rule::new(
            "Notification source resource",
            &[
                ("aws_codecommit_repository", &["arn"]),
                ("aws_codebuild_project", &["arn"]),
                ("aws_codedeploy_deployment_group", &["arn"]),
                ("aws_codepipeline", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_codestarnotifications_notification_rule", "target.address") => Rule::scalar_set(
            "Notification target",
            &[
                ("aws_sns_topic", &["arn"]),
                ("aws_chatbot_slack_channel_configuration", &["arn"]),
                ("aws_chatbot_teams_channel_configuration", &["arn"]),
            ],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    if r.attribute != "target.address" {
        return true;
    }
    let (block, _) = path.rsplit_once('.').unwrap();
    let field = format!("{block}.type");
    let kind = e.nodes[source].resource_type.as_str();
    (match e.text(r.target, &field) {
        Some("SNS") => kind == "aws_sns_topic",
        Some("AWSChatbotSlack") => kind == "aws_chatbot_slack_channel_configuration",
        Some("AWSChatbotMicrosoftTeams") => kind == "aws_chatbot_teams_channel_configuration",
        None => e.absent(r.target, &field) && kind == "aws_sns_topic",
        _ => false,
    }) && (kind != "aws_sns_topic"
        || crate::provider::aws::scope::scope(&e.nodes[r.target])
            .and_then(|scope| scope.region)
            .zip(
                crate::provider::aws::scope::scope(&e.nodes[source]).and_then(|scope| scope.region),
            )
            .is_some_and(|(left, right)| left == right))
}
