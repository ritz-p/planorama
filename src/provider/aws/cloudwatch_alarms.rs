use super::service_evidence::{eligible, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

fn alarm(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudwatch_metric_alarm" | "aws_cloudwatch_composite_alarm"
    )
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_cloudwatch_alarm_mute_rule", "mute_targets.alarm_names") => {
            Some("Configured mute targets")
        }
        ("aws_cloudwatch_composite_alarm", "actions_suppressor.alarm") => {
            Some("Configured suppressor alarm")
        }
        (_, "alarm_actions") if alarm(kind) => Some("Configured ALARM actions"),
        (_, "ok_actions") if alarm(kind) => Some("Configured OK actions"),
        (_, "insufficient_data_actions") if alarm(kind) => {
            Some("Configured INSUFFICIENT_DATA actions")
        }
        _ => None,
    }
}

fn arn_matches(node: &TerraformEntity, arn: &str, service: &str, prefix: &str) -> bool {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, actual_service, region, account, resource] = parts.as_slice() else {
        return false;
    };
    !partition.is_empty()
        && *actual_service == service
        && account.len() == 12
        && account.bytes().all(|b| b.is_ascii_digit())
        && resource.strip_prefix(prefix).is_some_and(|v| !v.is_empty())
        && super::scope::scope(node)
            .and_then(|s| s.region)
            .is_some_and(|r| r.0 == *region)
        && node
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            .is_none_or(|a| a.as_str() == *account)
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
        let suppressor = r.attribute == "actions_suppressor.alarm";
        let mute = r.attribute == "mute_targets.alarm_names";
        let output = if mute || (suppressor && r.source_attributes.contains("alarm_name")) {
            "alarm_name"
        } else {
            "arn"
        };
        if !r.source_attributes.contains(output) || (suppressor && r.sources.len() != 1) {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if (mute && value["mute_targets"].as_array().map(Vec::len) != Some(1))
            || (suppressor && value["actions_suppressor"].as_array().map(Vec::len) != Some(1))
        {
            continue;
        }
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, !suppressor) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                let endpoint = &nodes[source];
                let endpoint_kind = endpoint.resource_type.as_str();
                let id = known(evidence, source, output)?;
                let valid = if mute || suppressor {
                    alarm(endpoint_kind)
                        && (output == "alarm_name"
                            || arn_matches(endpoint, id, "cloudwatch", "alarm:"))
                } else {
                    let (service, prefix) = match endpoint_kind {
                        "aws_sns_topic" => ("sns", ""),
                        "aws_lambda_function" | "aws_lambda_alias" => ("lambda", "function:"),
                        "aws_autoscaling_policy" if kind == "aws_cloudwatch_metric_alarm" => {
                            ("autoscaling", "scalingPolicy:")
                        }
                        "aws_appautoscaling_policy" if kind == "aws_cloudwatch_metric_alarm" => {
                            ("autoscaling", "scalingPolicy:")
                        }
                        _ => return None,
                    };
                    arn_matches(endpoint, id, service, prefix)
                };
                valid.then_some(id)
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwatch_alarms.rs"]
mod tests;
