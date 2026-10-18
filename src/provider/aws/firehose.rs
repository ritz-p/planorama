use super::service_references::scalar;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/firehose.rs"]
mod tests;

pub(super) fn label(attribute: &str) -> Option<&'static str> {
    match attribute {
        "kinesis_source_configuration.kinesis_stream_arn" => Some("Kinesis source"),
        "extended_s3_configuration.bucket_arn" => Some("S3 delivery destination"),
        "extended_s3_configuration.s3_backup_configuration.bucket_arn" => {
            Some("S3 backup destination")
        }
        _ => None,
    }
}

fn known(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::Array(values) => values.iter().all(known),
        Value::Object(values) => values.values().all(known),
        _ => false,
    }
}

fn absent(value: &Value, unknown: &Value) -> bool {
    (value.is_null() || value.as_array().is_some_and(Vec::is_empty)) && known(unknown)
}

fn block<'a>(value: &'a Value, unknown: &'a Value, name: &str) -> Option<(&'a Value, &'a Value)> {
    if unknown == &Value::Bool(true) {
        return None;
    }
    let [value] = value[name].as_array()?.as_slice() else {
        return None;
    };
    let unknown = match &unknown[name] {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(values) if values.len() == 1 => &values[0],
        _ => return None,
    };
    (value.is_object()
        && (unknown.is_null() || unknown == &Value::Bool(false) || unknown.is_object()))
    .then_some((value, unknown))
}

fn alternatives_absent(
    value: &Value,
    unknown: &Value,
    is_alternative: impl Fn(&str) -> bool,
) -> bool {
    value
        .as_object()
        .into_iter()
        .flat_map(|v| v.keys())
        .chain(unknown.as_object().into_iter().flat_map(|v| v.keys()))
        .filter(|key| is_alternative(key))
        .all(|key| absent(&value[key], &unknown[key]))
}

pub(super) fn configuration_verified(attribute: &str, value: &Value, unknown: &Value) -> bool {
    if attribute == "kinesis_source_configuration.kinesis_stream_arn" {
        return block(value, unknown, "kinesis_source_configuration").is_some()
            && alternatives_absent(value, unknown, |key| {
                key.ends_with("_source_configuration") && key != "kinesis_source_configuration"
            });
    }
    if !matches!(
        attribute,
        "extended_s3_configuration.bucket_arn"
            | "extended_s3_configuration.s3_backup_configuration.bucket_arn"
    ) || scalar(value, unknown, &["destination"]) != Some("extended_s3")
        || !alternatives_absent(value, unknown, |key| {
            key.ends_with("_configuration")
                && !key.ends_with("_source_configuration")
                && key != "extended_s3_configuration"
        })
    {
        return false;
    }
    let Some((destination, unknown)) = block(value, unknown, "extended_s3_configuration") else {
        return false;
    };
    attribute == "extended_s3_configuration.bucket_arn"
        || (scalar(destination, unknown, &["s3_backup_mode"]) == Some("Enabled")
            && block(destination, unknown, "s3_backup_configuration").is_some())
}
