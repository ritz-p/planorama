fn bucket_configuration(resource: &str) -> bool {
    matches!(
        resource,
        "aws_s3_bucket_abac"
            | "aws_s3_bucket_accelerate_configuration"
            | "aws_s3_bucket_acl"
            | "aws_s3_bucket_cors_configuration"
            | "aws_s3_bucket_intelligent_tiering_configuration"
            | "aws_s3_bucket_lifecycle_configuration"
            | "aws_s3_bucket_object_lock_configuration"
            | "aws_s3_bucket_ownership_controls"
            | "aws_s3_bucket_public_access_block"
            | "aws_s3_bucket_request_payment_configuration"
            | "aws_s3_bucket_versioning"
            | "aws_s3_bucket_website_configuration"
            | "aws_s3_bucket_metric"
    )
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_notification.rs"]
mod notification_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_replication.rs"]
mod replication_tests;

pub(crate) fn indexed_resource(resource: &str) -> bool {
    matches!(
        resource,
        "aws_s3_bucket_notification"
            | "aws_s3_bucket_replication_configuration"
            | "aws_s3_bucket_metric"
    )
}

pub(crate) fn indexed_attribute(resource: &str, attribute: &str) -> Option<&'static str> {
    let parts: Vec<_> = attribute.split('.').collect();
    match (resource, parts.as_slice()) {
        ("aws_s3_bucket_metric", ["filter", "0", "access_point"]) => Some("filter.access_point"),
        (
            "aws_s3_bucket_replication_configuration",
            ["rule", rule, "destination", destination, "bucket"],
        ) if rule.parse::<usize>().is_ok() && destination.parse::<usize>().is_ok() => {
            Some("rule.destination.bucket")
        }
        ("aws_s3_bucket_notification", [block, index, field]) if index.parse::<usize>().is_ok() => {
            match (*block, *field) {
                ("queue", "queue_arn") => Some("queue.queue_arn"),
                ("topic", "topic_arn") => Some("topic.topic_arn"),
                ("lambda_function", "lambda_function_arn") => {
                    Some("lambda_function.lambda_function_arn")
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn list_shape(resource: &str, lengths: &[usize]) -> bool {
    match resource {
        "aws_s3_bucket_metric" => lengths == [1],
        "aws_s3_bucket_notification" => lengths.len() == 1,
        "aws_s3_bucket_replication_configuration" => matches!(lengths, [_, 1]),
        _ => false,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    if bucket_configuration(resource) && attribute == "bucket" {
        return Some("Configuration bucket reference");
    }
    match (
        resource,
        indexed_attribute(resource, attribute).unwrap_or(attribute),
    ) {
        ("aws_s3_bucket_metric", "filter.access_point") => Some("Metric access point reference"),
        ("aws_s3_bucket_policy", "bucket") => Some("Policy bucket reference"),
        ("aws_s3_bucket_replication_configuration", "bucket") => {
            Some("Replication source bucket reference")
        }
        ("aws_s3_bucket_replication_configuration", "rule.destination.bucket") => {
            Some("Replication destination reference")
        }
        ("aws_s3_bucket_notification", "bucket") => Some("Notification bucket reference"),
        ("aws_s3_bucket_notification", "queue.queue_arn") => Some("Notification queue reference"),
        ("aws_s3_bucket_notification", "topic.topic_arn") => Some("Notification topic reference"),
        ("aws_s3_bucket_notification", "lambda_function.lambda_function_arn") => {
            Some("Notification function reference")
        }
        _ => None,
    }
}

pub(super) fn source_fields(
    resource: &str,
    attribute: &str,
    source: &str,
) -> &'static [&'static str] {
    if bucket_configuration(resource) && attribute == "bucket" && source == "aws_s3_bucket" {
        return &["id", "bucket"];
    }
    if resource == "aws_s3_bucket_metric"
        && attribute == "bucket"
        && source == "aws_s3_directory_bucket"
    {
        return &["id", "bucket"];
    }
    let indexed = indexed_attribute(resource, attribute);
    match (resource, indexed.unwrap_or(attribute), source) {
        ("aws_s3_bucket_metric", "filter.access_point", "aws_s3_access_point")
            if indexed.is_some() =>
        {
            &["arn"]
        }
        (
            "aws_s3_bucket_policy"
            | "aws_s3_bucket_notification"
            | "aws_s3_bucket_replication_configuration",
            "bucket",
            "aws_s3_bucket",
        ) => &["id", "bucket"],
        ("aws_s3_bucket_notification", "queue.queue_arn", "aws_sqs_queue")
        | ("aws_s3_bucket_notification", "topic.topic_arn", "aws_sns_topic")
        | (
            "aws_s3_bucket_notification",
            "lambda_function.lambda_function_arn",
            "aws_lambda_function",
        )
        | ("aws_s3_bucket_replication_configuration", "rule.destination.bucket", "aws_s3_bucket")
            if indexed.is_some() =>
        {
            &["arn"]
        }
        _ => &[],
    }
}

pub(super) fn aggregate(
    raw: &crate::semantic::Input,
    reference: &crate::model::AttributeReference,
) -> bool {
    let resource = &raw.nodes[reference.target].resource_type;
    if !indexed_resource(resource)
        || !matches!(
            reference.attribute.as_str(),
            "filter.access_point"
                | "queue.queue_arn"
                | "topic.topic_arn"
                | "lambda_function.lambda_function_arn"
                | "rule.destination.bucket"
        )
    {
        return false;
    }
    raw.attributes.iter().any(|indexed| {
        indexed.target == reference.target
            && !indexed.indexed_block_lengths.is_empty()
            && indexed_attribute(resource, &indexed.attribute) == Some(reference.attribute.as_str())
    })
}
