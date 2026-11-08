use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/messaging_policies.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_sns_topic_policy" | "aws_sns_topic_data_protection_policy", "arn") => {
            Some("Policy topic")
        }
        (
            "aws_sqs_queue_policy"
            | "aws_sqs_queue_redrive_policy"
            | "aws_sqs_queue_redrive_allow_policy",
            "queue_url",
        ) => Some("Policy queue"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !super::service_evidence::eligible(nodes, r)
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let source = r.sources[0];
        let field = r.source_attributes.first().unwrap().as_str();
        let valid = match (r.attribute.as_str(), nodes[source].resource_type.as_str()) {
            ("arn", "aws_sns_topic") => field == "arn",
            ("queue_url", "aws_sqs_queue") => matches!(field, "url" | "id"),
            _ => false,
        };
        if valid {
            r.service_reference_verified =
                super::service_evidence::known(evidence, r.target, &r.attribute).is_some_and(
                    |value| super::service_evidence::known(evidence, source, field) == Some(value),
                );
        }
    }
}
