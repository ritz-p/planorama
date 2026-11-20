use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudtrail", "sns_topic_name") => Rule::new(
            "Trail notification topic",
            &[("aws_sns_topic", &["name"])],
            false,
            &[],
        ),
        ("aws_cloudtrail_event_data_store", "kms_key_id") => Rule::new(
            "Event data store encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id", "id"]),
                ("aws_kms_alias", &["arn", "name"]),
            ],
            false,
            &[],
        ),
        ("aws_controltower_baseline", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_controltower_control", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies);
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_controltower_baseline" | "aws_controltower_control", "target_identifier") => e
            .text(r.target, path)
            .and_then(organizations_ou_account)
            .is_some_and(|account| {
                e.nodes[source]
                    .provider_configuration
                    .as_ref()
                    .and_then(|config| config.account.as_ref())
                    .is_none_or(|owner| owner.as_str() == account)
            }),
        _ => true,
    }
}

fn organizations_ou_account(arn: &str) -> Option<&str> {
    let mut fields = arn.splitn(6, ':');
    match (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) {
        (
            Some("arn"),
            Some("aws" | "aws-us-gov" | "aws-cn"),
            Some("organizations"),
            Some(""),
            Some(account),
            Some(resource),
        ) if crate::model::AwsAccountScope::new(account).is_some()
            && resource
                .strip_prefix("ou/o-")
                .and_then(|path| path.split_once("/ou-"))
                .is_some_and(|(org, ou)| {
                    !org.is_empty() && !ou.is_empty() && !ou.contains('/')
                }) =>
        {
            Some(account)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/operations.rs"]
mod tests;
