use crate::model::{
    Action, AttributeReference, EntityMode, ProviderConfiguration, RegionScope, TerraformEntity,
};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/scheduler_pipes.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_scheduler_schedule", "group_name") => Some("Schedule group membership"),
        ("aws_scheduler_schedule", "target.arn") => Some("Schedule target"),
        ("aws_scheduler_schedule", "target.role_arn") => Some("Schedule execution role"),
        ("aws_pipes_pipe", "source") => Some("Pipe source"),
        ("aws_pipes_pipe", "enrichment") => Some("Pipe enrichment"),
        ("aws_pipes_pipe", "target") => Some("Pipe target"),
        ("aws_pipes_pipe", "role_arn") => Some("Pipe execution role"),
        _ => None,
    }
}

fn fields(resource: &str, attribute: &str, endpoint: &str) -> &'static [&'static str] {
    match (resource, attribute, endpoint) {
        ("aws_scheduler_schedule", "group_name", "aws_scheduler_schedule_group") => &["name", "id"],
        ("aws_scheduler_schedule", "target.role_arn", "aws_iam_role")
        | ("aws_pipes_pipe", "role_arn", "aws_iam_role") => &["arn"],
        ("aws_pipes_pipe", "source", "aws_sqs_queue" | "aws_kinesis_stream")
        | ("aws_pipes_pipe", "enrichment", "aws_lambda_function") => &["arn"],
        ("aws_scheduler_schedule", "target.arn", kind) | ("aws_pipes_pipe", "target", kind)
            if matches!(
                kind,
                "aws_sqs_queue"
                    | "aws_sns_topic"
                    | "aws_lambda_function"
                    | "aws_kinesis_stream"
                    | "aws_sfn_state_machine"
                    | "aws_cloudwatch_event_bus"
            ) =>
        {
            &["arn"]
        }
        _ => &[],
    }
}

pub(super) fn bindings(
    consumer: Option<&ProviderConfiguration>,
    endpoint: Option<&ProviderConfiguration>,
    consumer_region: Option<&RegionScope>,
    endpoint_region: Option<&RegionScope>,
    role: bool,
) -> bool {
    let (Some(left), Some(right)) = (consumer, endpoint) else {
        return false;
    };
    !left.account_conflict
        && !right.account_conflict
        && match (&left.account, &right.account) {
            (Some(a), Some(b)) => a == b,
            (None, None) => left == right,
            _ => false,
        }
        && consumer_region.is_some()
        && (role || consumer_region == endpoint_region)
}

fn scalar<'a>(value: &'a Value, unknown: &Value, path: &[&str]) -> Option<&'a str> {
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        let mask = match unknown {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(masks) if masks.len() == 1 => &masks[0],
            _ => return None,
        };
        return scalar(value, mask, path);
    }
    if let Some((field, rest)) = path.split_first() {
        if !value.is_object()
            || !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        return scalar(&value[*field], &unknown[*field], rest);
    }
    matches!(unknown, Value::Null | Value::Bool(false))
        .then(|| value.as_str().filter(|v| !v.is_empty()))
        .flatten()
}

fn source_parameters(value: &Value, unknown: &Value, endpoint: &str) -> bool {
    let expected = match endpoint {
        "aws_sqs_queue" => "sqs_queue_parameters",
        "aws_kinesis_stream" => "kinesis_stream_parameters",
        _ => return false,
    };
    let parameter = &value["source_parameters"];
    let mask = &unknown["source_parameters"];
    if parameter.is_null() || parameter.as_array().is_some_and(Vec::is_empty) {
        return matches!(mask, Value::Null | Value::Bool(false));
    }
    let Some(parameters) = parameter.as_array() else {
        return false;
    };
    let [parameter] = parameters.as_slice() else {
        return false;
    };
    let mask = match mask {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(masks) if masks.len() == 1 => &masks[0],
        _ => return false,
    };
    let Some(parameters) = parameter.as_object() else {
        return false;
    };
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    parameters
        .keys()
        .chain(mask.as_object().into_iter().flat_map(|m| m.keys()))
        .filter(|key| key.ends_with("_parameters") && key.as_str() != expected)
        .all(|key| {
            (parameter[key].is_null() || parameter[key].as_array().is_some_and(Vec::is_empty))
                && matches!(&mask[key], Value::Null | Value::Bool(false))
        })
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let [source] = reference.sources.as_slice() else {
        return false;
    };
    let node = &nodes[reference.target];
    let endpoint = &nodes[*source];
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    if !current(node)
        || !current(endpoint)
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || reference.source_attributes.len() != 1
    {
        return false;
    }
    if !bindings(
        node.provider_configuration.as_ref(),
        endpoint.provider_configuration.as_ref(),
        super::scope::scope(node)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        super::scope::scope(endpoint)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        endpoint.resource_type == "aws_iam_role",
    ) {
        return false;
    }
    let (value, unknown) = evidence[reference.target];
    let (target, target_unknown) = evidence[*source];
    let path = reference.attribute.split('.').collect::<Vec<_>>();
    let actual = scalar(value, unknown, &path);
    fields(
        &node.resource_type,
        &reference.attribute,
        &endpoint.resource_type,
    )
    .iter()
    .any(|field| {
        reference.source_attributes.contains(*field)
            && actual.is_some()
            && actual == scalar(target, target_unknown, &[*field])
    }) && (node.resource_type != "aws_pipes_pipe"
        || reference.attribute != "source"
        || source_parameters(value, unknown, &endpoint.resource_type))
}
