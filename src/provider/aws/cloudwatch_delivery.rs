use super::service_evidence::known;
use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwatch_delivery.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_cloudwatch_log_delivery", "delivery_source_name") => {
            Some("Configured delivery source")
        }
        ("aws_cloudwatch_log_delivery", "delivery_destination_arn") => {
            Some("Configured delivery destination")
        }
        (
            "aws_cloudwatch_log_delivery_destination",
            "delivery_destination_configuration.destination_resource_arn",
        ) => Some("Configured delivery endpoint"),
        ("aws_cloudwatch_log_delivery_destination_policy", "delivery_destination_name") => {
            Some("Delivery policy destination")
        }
        ("aws_cloudwatch_log_delivery_source", "resource_arn") => Some("Configured log producer"),
        ("aws_cloudwatch_log_destination", "target_arn") => Some("Configured Kinesis target"),
        ("aws_cloudwatch_log_destination_policy", "destination_name") => {
            Some("Subscription policy destination")
        }
        ("aws_cloudwatch_log_subscription_filter", "log_group_name") => {
            Some("Configured log group")
        }
        ("aws_cloudwatch_log_subscription_filter", "destination_arn") => {
            Some("Configured subscription destination")
        }
        (
            "aws_cloudwatch_log_destination" | "aws_cloudwatch_log_subscription_filter",
            "role_arn",
        ) => Some("Logging role reference"),
        _ => None,
    }
}

fn output(field: &str, kind: &str) -> Option<&'static str> {
    match (field, kind) {
        ("delivery_source_name", "aws_cloudwatch_log_delivery_source")
        | ("delivery_destination_name", "aws_cloudwatch_log_delivery_destination")
        | ("destination_name", "aws_cloudwatch_log_destination")
        | ("log_group_name", "aws_cloudwatch_log_group") => Some("name"),
        ("delivery_destination_arn", "aws_cloudwatch_log_delivery_destination")
        | (
            "delivery_destination_configuration.destination_resource_arn",
            "aws_s3_bucket" | "aws_cloudwatch_log_group" | "aws_kinesis_firehose_delivery_stream",
        )
        | ("resource_arn", "aws_cloudfront_distribution" | "aws_bedrockagent_knowledge_base")
        | ("target_arn", "aws_kinesis_stream")
        | (
            "destination_arn",
            "aws_lambda_function"
            | "aws_kinesis_stream"
            | "aws_kinesis_firehose_delivery_stream"
            | "aws_cloudwatch_log_destination",
        )
        | ("role_arn", "aws_iam_role") => Some("arn"),
        _ => None,
    }
}

fn binding(
    left: Option<&crate::model::ProviderConfiguration>,
    right: Option<&crate::model::ProviderConfiguration>,
    left_region: Option<&crate::model::RegionScope>,
    right_region: Option<&crate::model::RegionScope>,
    endpoint: &str,
    field: &str,
) -> bool {
    let cross_account_destination = matches!(
        (field, endpoint),
        ("destination_arn", "aws_cloudwatch_log_destination")
            | (
                "delivery_destination_arn",
                "aws_cloudwatch_log_delivery_destination"
            )
    );
    if cross_account_destination {
        if let (Some(left), Some(right)) = (left, right) {
            if let (Some(a), Some(b)) = (&left.account, &right.account) {
                if a != b {
                    return !left.account_conflict
                        && !right.account_conflict
                        && left_region.is_some()
                        && left_region == right_region;
                }
            }
        }
    }
    super::eventbridge::bindings(
        left,
        right,
        left_region,
        right_region,
        matches!(
            endpoint,
            "aws_iam_role" | "aws_s3_bucket" | "aws_cloudfront_distribution"
        ),
    )
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let consumer = &raw.nodes[r.target];
    r.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        binding(
            consumer.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            consumer
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            &endpoint.resource_type,
            &r.attribute,
        )
    })
}
fn discriminator(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let source_kind = nodes[r.sources[0]].resource_type.as_str();
    match r.attribute.as_str() {
        "delivery_destination_configuration.destination_resource_arn" => {
            let expected = match source_kind {
                "aws_s3_bucket" => "S3",
                "aws_cloudwatch_log_group" => "CWL",
                "aws_kinesis_firehose_delivery_stream" => "FH",
                _ => return false,
            };
            let (value, _) = evidence[r.target];
            value["delivery_destination_configuration"]
                .as_array()
                .is_some_and(|entries| entries.len() == 1)
                && known(evidence, r.target, "delivery_destination_type") == Some(expected)
        }
        "resource_arn" => {
            let (service, log_type) = match source_kind {
                "aws_cloudfront_distribution" => ("cloudfront", "ACCESS_LOGS"),
                "aws_bedrockagent_knowledge_base" => ("bedrock", "APPLICATION_LOGS"),
                _ => return false,
            };
            known(evidence, r.target, "log_type") == Some(log_type)
                && known(evidence, r.sources[0], "arn")
                    .is_some_and(|arn| arn.split(':').nth(2) == Some(service))
        }
        "role_arn" if nodes[r.target].resource_type == "aws_cloudwatch_log_subscription_filter" => {
            known(evidence, r.target, "destination_arn")
                .is_some_and(|arn| matches!(arn.split(':').nth(2), Some("kinesis" | "firehose")))
        }
        "destination_arn"
            if matches!(
                source_kind,
                "aws_lambda_function" | "aws_cloudwatch_log_destination"
            ) =>
        {
            let (value, mask) = evidence[r.target];
            value["role_arn"].is_null()
                && matches!(&mask["role_arn"], Value::Null | Value::Bool(false))
        }
        _ => true,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
    };
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !current(&nodes[r.target])
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let source = r.sources[0];
        let Some(output) = output(&r.attribute, &nodes[source].resource_type) else {
            continue;
        };
        if !current(&nodes[source])
            || !r.source_attributes.contains(output)
            || !binding(
                nodes[r.target].provider_configuration.as_ref(),
                nodes[source].provider_configuration.as_ref(),
                super::scope::scope(&nodes[r.target])
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                super::scope::scope(&nodes[source])
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                &nodes[source].resource_type,
                &r.attribute,
            )
            || !discriminator(nodes, evidence, r)
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        r.service_reference_verified = super::streaming::values(value, mask, &r.attribute, false)
            .is_some_and(|values| {
                known(evidence, source, output)
                    .is_some_and(|expected| values.len() == 1 && values.contains(expected))
            });
    }
}
