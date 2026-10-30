use super::service_evidence::{eligible, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_cloudwatch_log_anomaly_detector", "log_group_arn_list")
        | ("aws_cloudwatch_query_definition", "log_group_names") => Some("Configured log groups"),
        (
            "aws_cloudwatch_log_data_protection_policy"
            | "aws_cloudwatch_log_index_policy"
            | "aws_cloudwatch_log_metric_filter"
            | "aws_cloudwatch_log_stream",
            "log_group_name",
        )
        | ("aws_cloudwatch_log_resource_policy", "resource_arn")
        | ("aws_cloudwatch_log_transformer", "log_group_arn") => Some("Configured log group"),
        ("aws_cloudwatch_log_group" | "aws_cloudwatch_log_anomaly_detector", "kms_key_id") => {
            Some("Encryption key reference")
        }
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !eligible(nodes, r) || r.source_attributes.len() != 1 {
            continue;
        }
        let collection = matches!(
            r.attribute.as_str(),
            "log_group_names" | "log_group_arn_list"
        );
        if (!collection || kind == "aws_cloudwatch_log_anomaly_detector") && r.sources.len() != 1 {
            continue;
        }
        let (target_kind, output) = match r.attribute.as_str() {
            "log_group_name" | "log_group_names" => ("aws_cloudwatch_log_group", "name"),
            "kms_key_id" => (
                "aws_kms_key",
                if kind == "aws_cloudwatch_log_anomaly_detector"
                    && r.source_attributes.contains("key_id")
                {
                    "key_id"
                } else {
                    "arn"
                },
            ),
            _ => ("aws_cloudwatch_log_group", "arn"),
        };
        if !r.source_attributes.contains(output) {
            continue;
        }
        if kind == "aws_cloudwatch_log_resource_policy"
            && known(evidence, r.target, "policy_scope") != Some("RESOURCE")
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if kind == "aws_cloudwatch_log_anomaly_detector"
            && collection
            && value[&r.attribute].as_array().map(Vec::len) != Some(1)
        {
            continue;
        }
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == target_kind)
                    .then(|| known(evidence, source, output))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwatch_logs.rs"]
mod tests;
