use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront_observability.rs"]
mod tests;

const SUBSCRIPTION: &str = "aws_cloudfront_monitoring_subscription";
const LOG: &str = "aws_cloudfront_realtime_log_config";
const ROLE: &str = "endpoint.kinesis_stream_config.role_arn";
const STREAM: &str = "endpoint.kinesis_stream_config.stream_arn";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (SUBSCRIPTION, "distribution_id") => Some("Monitoring distribution"),
        (LOG, ROLE) => Some("Realtime log IAM role"),
        (LOG, STREAM) => Some("Realtime log Kinesis stream"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = BTreeMap::new();
    for r in refs.iter() {
        *counts.entry((r.target, r.attribute.clone())).or_insert(0) += 1;
    }
    for r in refs {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1 {
            continue;
        }
        let (kind, field) = match (node.resource_type.as_str(), r.attribute.as_str()) {
            (SUBSCRIPTION, _) => ("aws_cloudfront_distribution", "id"),
            (LOG, ROLE) => ("aws_iam_role", "arn"),
            (LOG, STREAM) => ("aws_kinesis_stream", "arn"),
            _ => continue,
        };
        let Some(s) = super::route53::endpoint(nodes, r, kind, &[field]) else {
            continue;
        };
        if !super::route53::configured(node, &nodes[s]) {
            continue;
        }
        if node.resource_type == LOG {
            let (v, _) = evidence[r.target];
            if v["endpoint"].as_array().is_none_or(|a| a.len() != 1)
                || v["endpoint"][0]["kinesis_stream_config"]
                    .as_array()
                    .is_none_or(|a| a.len() != 1)
                || super::appsync::scalar(evidence, r.target, "endpoint.stream_type")
                    != Some("Kinesis")
                || (r.attribute == STREAM
                    && super::scope::scope(&nodes[s])
                        .and_then(|s| s.region)
                        .is_none())
            {
                continue;
            }
        }
        let value = super::appsync::scalar(evidence, r.target, &r.attribute);
        r.service_reference_verified =
            value.is_some() && value == super::service_evidence::known(evidence, s, field);
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || super::route53::bindings(raw, r)
}
