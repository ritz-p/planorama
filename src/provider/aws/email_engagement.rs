use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_mailmanager_archive", "kms_key_arn") => {
            Rule::new("Archive key", &[("aws_kms_key", &["arn"])], false, &[])
        }
        ("aws_mailmanager_ingress_point", "rule_set_id") => Rule::new(
            "Mail rule set",
            &[("aws_mailmanager_rule_set", &["id"])],
            false,
            &[],
        ),
        ("aws_mailmanager_ingress_point", "traffic_policy_id") => Rule::new(
            "Mail traffic policy",
            &[("aws_mailmanager_traffic_policy", &["id"])],
            false,
            &[],
        ),
        (
            "aws_mailmanager_ingress_point",
            "network_configuration.private_network_configuration.vpc_endpoint_id",
        ) => Rule::new(
            "Private ingress endpoint",
            &[("aws_vpc_endpoint", &["id"])],
            false,
            &[
                "network_configuration",
                "network_configuration.private_network_configuration",
            ],
        ),
        ("aws_mailmanager_ingress_point", "ingress_point_configuration.secret_arn") => Rule::new(
            "Ingress secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &["ingress_point_configuration"],
        ),
        ("aws_mailmanager_relay", "authentication.secret_arn") => Rule::new(
            "Relay secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &["authentication"],
        ),
        ("aws_mailmanager_rule_set", "rule.action.archive.target_archive") => Rule::new(
            "Mail archive",
            &[("aws_mailmanager_archive", &["id"])],
            false,
            &["rule", "rule.action", "rule.action.archive"],
        ),
        ("aws_mailmanager_rule_set", "rule.action.relay.relay") => Rule::new(
            "Mail relay",
            &[("aws_mailmanager_relay", &["id"])],
            false,
            &["rule", "rule.action", "rule.action.relay"],
        ),
        ("aws_mailmanager_rule_set", "rule.action.invoke_lambda.function_arn") => Rule::new(
            "Lambda function",
            &[("aws_lambda_function", &["arn"])],
            false,
            &["rule", "rule.action", "rule.action.invoke_lambda"],
        ),
        ("aws_mailmanager_rule_set", "rule.action.publish_to_sns.topic_arn") => Rule::new(
            "SNS topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &["rule", "rule.action", "rule.action.publish_to_sns"],
        ),
        ("aws_mailmanager_rule_set", "rule.action.write_to_s3.s3_bucket") => Rule::new(
            "S3 bucket",
            &[("aws_s3_bucket", &["bucket"])],
            false,
            &["rule", "rule.action", "rule.action.write_to_s3"],
        ),
        ("aws_pinpoint_app", "campaign_hook.lambda_function_name") => Rule::new(
            "Campaign hook function",
            &[("aws_lambda_function", &["function_name", "arn"])],
            false,
            &["campaign_hook"],
        ),
        ("aws_pinpoint_email_channel", "configuration_set") => Rule::new(
            "SES configuration set",
            &[
                ("aws_ses_configuration_set", &["arn"]),
                ("aws_sesv2_configuration_set", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_pinpoint_email_channel", "identity") => Rule::new(
            "SES identity",
            &[
                ("aws_ses_domain_identity", &["arn"]),
                ("aws_ses_email_identity", &["arn"]),
                ("aws_sesv2_email_identity", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_pinpoint_email_channel", "role_arn") => Rule::new(
            "Email sending role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_pinpoint_email_channel", "orchestration_sending_role_arn") => Rule::new(
            "Orchestration sending role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_pinpoint_event_stream", "destination_stream_arn") => Rule::new(
            "Event stream destination",
            &[
                ("aws_kinesis_stream", &["arn"]),
                ("aws_kinesis_firehose_delivery_stream", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_pinpoint_event_stream", "role_arn") => Rule::new(
            "Event stream role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_pinpoint_adm_channel"
            | "aws_pinpoint_apns_channel"
            | "aws_pinpoint_apns_sandbox_channel"
            | "aws_pinpoint_apns_voip_channel"
            | "aws_pinpoint_apns_voip_sandbox_channel"
            | "aws_pinpoint_baidu_channel"
            | "aws_pinpoint_email_channel"
            | "aws_pinpoint_event_stream"
            | "aws_pinpoint_gcm_channel"
            | "aws_pinpoint_sms_channel",
            "application_id",
        ) => Rule::new(
            "Pinpoint application",
            &[("aws_pinpoint_app", &["application_id", "id"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_event_destination", "configuration_set_name") => Rule::new(
            "SMS configuration set",
            &[("aws_pinpointsmsvoicev2_configuration_set", &["name"])],
            false,
            &[],
        ),
        (
            "aws_pinpointsmsvoicev2_event_destination",
            "cloudwatch_logs_destination.log_group_arn",
        ) => Rule::new(
            "CloudWatch log group",
            &[("aws_cloudwatch_log_group", &["arn"])],
            false,
            &["cloudwatch_logs_destination"],
        ),
        (
            "aws_pinpointsmsvoicev2_event_destination",
            "kinesis_firehose_destination.delivery_stream_arn",
        ) => Rule::new(
            "Firehose delivery stream",
            &[("aws_kinesis_firehose_delivery_stream", &["arn"])],
            false,
            &["kinesis_firehose_destination"],
        ),
        ("aws_pinpointsmsvoicev2_event_destination", "sns_destination.topic_arn") => Rule::new(
            "SNS topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &["sns_destination"],
        ),
        ("aws_pinpointsmsvoicev2_keyword", "origination_identity_arn") => Rule::new(
            "Keyword origination identity",
            &[
                ("aws_pinpointsmsvoicev2_phone_number", &["arn"]),
                ("aws_pinpointsmsvoicev2_pool", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_phone_number", "opt_out_list_name") => Rule::new(
            "Opt-out list",
            &[("aws_pinpointsmsvoicev2_opt_out_list", &["name"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_phone_number", "two_way_channel_arn") => Rule::new(
            "Inbound SNS topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_phone_number", "two_way_channel_role") => Rule::new(
            "Inbound messaging role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_pool", "origination_identities") => Rule::new(
            "Pool origination identities",
            &[
                ("aws_pinpointsmsvoicev2_phone_number", &["arn"]),
                ("aws_pinpointsmsvoicev2_sender_id", &["arn"]),
            ],
            true,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_pool", "opt_out_list_name") => Rule::new(
            "Opt-out list",
            &[("aws_pinpointsmsvoicev2_opt_out_list", &["name"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_pool", "two_way_channel_arn") => Rule::new(
            "Inbound SNS topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[],
        ),
        ("aws_pinpointsmsvoicev2_resource_policy", "resource_arn") => Rule::new(
            "SMS policy resource",
            &[
                ("aws_pinpointsmsvoicev2_phone_number", &["arn"]),
                ("aws_pinpointsmsvoicev2_pool", &["arn"]),
                ("aws_pinpointsmsvoicev2_sender_id", &["arn"]),
                ("aws_pinpointsmsvoicev2_opt_out_list", &["arn"]),
            ],
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

#[cfg(test)]
#[path = "../../../tests/unit/semantic/mailmanager.rs"]
mod tests;
