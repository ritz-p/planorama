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
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["rule", "rule.action", "rule.action.write_to_s3"],
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
