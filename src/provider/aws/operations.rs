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

fn qualifies(_: &Evidence<'_>, _: &AttributeReference, _: usize, _: &str) -> bool {
    true
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/operations.rs"]
mod tests;
