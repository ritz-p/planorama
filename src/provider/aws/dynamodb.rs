use crate::model::{AttributeReference, ProviderConfiguration, RegionScope, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/dynamodb.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_dynamodb_table_replica", "global_table_arn") => Some("V2 table membership"),
        ("aws_dynamodb_global_table", "name") => Some("Legacy V1 table membership"),
        (
            "aws_dynamodb_contributor_insights"
            | "aws_dynamodb_global_secondary_index"
            | "aws_dynamodb_kinesis_streaming_destination",
            "table_name",
        ) => Some("Configured table"),
        ("aws_dynamodb_resource_policy" | "aws_dynamodb_tag", "resource_arn") => {
            Some("Configured table")
        }
        ("aws_dynamodb_kinesis_streaming_destination", "stream_arn") => {
            Some("Configured Kinesis stream")
        }
        _ => None,
    }
}

pub(super) fn source_fields(
    resource: &str,
    attribute: &str,
    source: &str,
) -> &'static [&'static str] {
    match (resource, attribute, source) {
        ("aws_dynamodb_table_replica", "global_table_arn", "aws_dynamodb_table") => &["arn"],
        ("aws_dynamodb_global_table", "name", "aws_dynamodb_table")
        | (
            "aws_dynamodb_contributor_insights"
            | "aws_dynamodb_global_secondary_index"
            | "aws_dynamodb_kinesis_streaming_destination",
            "table_name",
            "aws_dynamodb_table",
        ) => &["name", "id"],
        (
            "aws_dynamodb_resource_policy" | "aws_dynamodb_tag",
            "resource_arn",
            "aws_dynamodb_table" | "aws_dynamodb_table_replica",
        ) => &["arn"],
        ("aws_dynamodb_kinesis_streaming_destination", "stream_arn", "aws_kinesis_stream") => {
            &["arn"]
        }
        _ => &[],
    }
}

pub(super) fn bindings(
    resource: &str,
    consumer: Option<&ProviderConfiguration>,
    endpoint: Option<&ProviderConfiguration>,
    consumer_region: Option<&RegionScope>,
    endpoint_region: Option<&RegionScope>,
) -> bool {
    let (Some(consumer), Some(endpoint)) = (consumer, endpoint) else {
        return false;
    };
    if consumer.account_conflict || endpoint.account_conflict {
        return false;
    }
    let same_account = match (&consumer.account, &endpoint.account) {
        (Some(left), Some(right)) => left == right,
        (None, None) => consumer == endpoint,
        _ => false,
    };
    same_account && regions_match(resource, consumer_region, endpoint_region)
}

fn regions_match(
    resource: &str,
    consumer_region: Option<&RegionScope>,
    endpoint_region: Option<&RegionScope>,
) -> bool {
    endpoint_region.is_some()
        && match resource {
            "aws_dynamodb_global_table" => true,
            "aws_dynamodb_table_replica" => {
                consumer_region.is_some() && consumer_region != endpoint_region
            }
            _ => consumer_region.is_some() && consumer_region == endpoint_region,
        }
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    if label(&node.resource_type, &reference.attribute).is_none() {
        return true;
    }
    let [source] = reference.sources.as_slice() else {
        return false;
    };
    let endpoint = &nodes[*source];
    if super::service_evidence::known(evidence, reference.target, &reference.attribute).is_none()
        || reference
            .source_attributes
            .iter()
            .any(|field| super::service_evidence::known(evidence, *source, field).is_none())
    {
        return false;
    }
    let left = super::scope::scope(node);
    let right = super::scope::scope(endpoint);
    let consumer_region = left.as_ref().and_then(|s| s.region.as_ref());
    let endpoint_region = right.as_ref().and_then(|s| s.region.as_ref());
    if !regions_match(&node.resource_type, consumer_region, endpoint_region) {
        return false;
    }
    if node.resource_type != "aws_dynamodb_global_table" {
        return true;
    }
    let (value, unknown) = evidence[reference.target];
    let Some(replicas) = value["replica"].as_array() else {
        return false;
    };
    let mask = &unknown["replica"];
    if !matches!(mask, Value::Null | Value::Bool(false))
        && mask.as_array().is_none_or(|m| m.len() != replicas.len())
    {
        return false;
    }
    let mut regions = std::collections::BTreeSet::new();
    for (index, replica) in replicas.iter().enumerate() {
        let Some(region) =
            super::service_evidence::known(&[(replica, &mask[index])], 0, "region_name")
        else {
            return false;
        };
        if !regions.insert(region) {
            return false;
        }
    }
    right
        .as_ref()
        .and_then(|s| s.region.as_ref())
        .is_some_and(|region| regions.contains(region.0.as_str()))
}
