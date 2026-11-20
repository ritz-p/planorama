use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_devopsagent_agent_space"
            | "aws_devopsguru_notification_channel"
            | "aws_devopsguru_resource_collection"
            | "aws_devopsguru_service_integration"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_devopsagent_agent_space", "kms_key_arn") => Rule::new(
            "Agent Space encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_devopsguru_notification_channel", "sns.topic_arn") => Rule::new(
            "DevOps Guru notification topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[],
        ),
        ("aws_devopsguru_resource_collection", "cloudformation.stack_names") => Rule::new(
            "DevOps Guru monitored stacks",
            &[("aws_cloudformation_stack", &["name"])],
            true,
            &[],
        ),
        ("aws_devopsguru_service_integration", "kms_server_side_encryption.kms_key_id") => {
            Rule::new(
                "DevOps Guru encryption key",
                &[("aws_kms_key", &["arn", "id", "key_id"])],
                false,
                &[],
            )
        }
        _ => return None,
    })
}

pub(super) fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    match r.attribute.as_str() {
        "cloudformation.stack_names" => {
            e.text(r.target, "type") == Some("CLOUD_FORMATION")
                && e.text(source, "name")
                    .is_some_and(|name| !name.contains('*'))
        }
        "kms_server_side_encryption.kms_key_id" => {
            e.text(r.target, "kms_server_side_encryption.0.opt_in_status") == Some("ENABLED")
                && e.text(r.target, "kms_server_side_encryption.0.type")
                    == Some("CUSTOMER_MANAGED_KEY")
        }
        _ => true,
    }
}
