use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_events.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    if resource != "aws_lambda_event_source_mapping" {
        return None;
    }
    match attribute {
        "destination_config.on_failure.destination_arn" => Some("Failed-record destination"),
        "kms_key_arn" => Some("Filter encryption key"),
        "source_access_configuration.uri" => Some("Event source access configuration"),
        "amazon_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri"
        | "self_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri" => {
            Some("Kafka schema registry")
        }
        _ => None,
    }
}

pub(super) fn extended(raw: &Input, target: usize) -> bool {
    raw.attributes
        .iter()
        .filter(|r| r.target == target)
        .any(|r| {
            (r.attribute == "event_source_arn"
                && r.sources.iter().any(|&s| {
                    !matches!(
                        raw.nodes[s].resource_type.as_str(),
                        "aws_sqs_queue" | "aws_kinesis_stream"
                    )
                }))
                || (r.attribute == "function_name"
                    && (r.source_attributes.contains("qualified_arn")
                        || r.sources
                            .iter()
                            .any(|&s| raw.nodes[s].resource_type == "aws_lambda_alias")))
        })
}

pub(super) fn endpoint(raw: &Input, r: &AttributeReference) -> bool {
    r.service_reference_verified && super::streaming::bindings(raw, r)
}

pub(super) fn function(raw: &Input, endpoint: usize) -> Option<usize> {
    if raw.nodes[endpoint].resource_type != "aws_lambda_alias" {
        return Some(endpoint);
    }
    let mut refs = raw
        .attributes
        .iter()
        .filter(|r| r.target == endpoint && r.attribute == "function_name");
    let r = refs.next()?;
    let [parent] = r.sources.as_slice() else {
        return None;
    };
    (refs.next().is_none() && r.service_reference_verified && super::streaming::bindings(raw, r))
        .then_some(*parent)
}

fn scalar<'a>(evidence: &[(&'a Value, &'a Value)], index: usize, path: &str) -> Option<&'a str> {
    let (value, mask) = evidence[index];
    super::streaming::values(value, mask, path, false)?
        .into_iter()
        .next()
}

fn source_field(kind: &str) -> Option<&'static str> {
    match kind {
        "aws_dynamodb_table" => Some("stream_arn"),
        "aws_sqs_queue"
        | "aws_kinesis_stream"
        | "aws_msk_cluster"
        | "aws_msk_serverless_cluster"
        | "aws_mq_broker"
        | "aws_docdb_cluster" => Some("arn"),
        _ => None,
    }
}

fn matches(
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
    field: &str,
) -> bool {
    r.source_attributes.len() == 1
        && r.source_attributes.contains(field)
        && scalar(evidence, r.target, &r.attribute)
            .is_some_and(|value| Some(value) == scalar(evidence, source, field))
}

fn qualified(evidence: &[(&Value, &Value)], source: usize) -> bool {
    let Some(version) = scalar(evidence, source, "version") else {
        return false;
    };
    version.parse::<u64>().is_ok_and(|v| v > 0)
        && scalar(evidence, source, "arn")
            .zip(scalar(evidence, source, "qualified_arn"))
            .is_some_and(|(arn, qualified)| qualified == format!("{arn}:{version}"))
}

fn absent(value: &Value, mask: &Value, key: &str) -> bool {
    matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&mask[key], Value::Null | Value::Bool(false))
        && (value[key].is_null() || value[key].as_array().is_some_and(Vec::is_empty))
}

fn shape(value: &Value, path: &str) -> bool {
    let Some((prefix, _)) = path.rsplit_once('.') else {
        return true;
    };
    let mut at = value;
    prefix.split('.').all(|field| {
        let Some(entries) = at[field].as_array() else {
            return false;
        };
        let [entry] = entries.as_slice() else {
            return false;
        };
        at = entry;
        true
    })
}

fn source_context(value: &Value, mask: &Value, kind: &str) -> bool {
    absent(value, mask, "self_managed_event_source")
        && absent(value, mask, "self_managed_kafka_event_source_config")
        && (kind == "aws_docdb_cluster" || absent(value, mask, "document_db_event_source_config"))
        && (matches!(kind, "aws_msk_cluster" | "aws_msk_serverless_cluster")
            || absent(value, mask, "amazon_managed_kafka_event_source_config"))
}

fn kafka(value: &Value, mask: &Value) -> bool {
    absent(value, mask, "event_source_arn")
        && absent(value, mask, "amazon_managed_kafka_event_source_config")
        && absent(value, mask, "document_db_event_source_config")
        && value["self_managed_event_source"]
            .as_array()
            .is_some_and(|v| v.len() == 1)
        && matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(
            &mask["self_managed_event_source"],
            Value::Null | Value::Bool(false)
        )
}

fn event_kind<'a>(
    nodes: &'a [TerraformEntity],
    refs: &[AttributeReference],
    target: usize,
    evidence: &[(&Value, &Value)],
) -> Option<&'a str> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == "event_source_arn");
    let r = found.next()?;
    (found.next().is_none() && base(nodes, refs, r, evidence))
        .then(|| nodes[r.sources[0]].resource_type.as_str())
}

fn base(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    if !super::service_evidence::eligible(nodes, r) || r.sources.len() != 1 {
        return false;
    }
    let source = r.sources[0];
    let (value, mask) = evidence[r.target];
    if !shape(value, &r.attribute) {
        return false;
    }
    match r.attribute.as_str() {
        "event_source_arn" => {
            source_context(value, mask, &nodes[source].resource_type)
                && source_field(&nodes[source].resource_type)
                    .is_some_and(|field| matches(evidence, r, source, field))
        }
        "function_name" => match nodes[source].resource_type.as_str() {
            "aws_lambda_function" => {
                matches(evidence, r, source, "function_name")
                    || matches(evidence, r, source, "arn")
                    || (matches(evidence, r, source, "qualified_arn")
                        && qualified(evidence, source))
            }
            "aws_lambda_alias" => {
                matches(evidence, r, source, "arn")
                    && refs
                        .iter()
                        .find(|p| {
                            p.target == source
                                && p.attribute == "function_name"
                                && p.service_reference_verified
                        })
                        .is_some_and(|p| {
                            let [parent] = p.sources.as_slice() else {
                                return false;
                            };
                            scalar(evidence, *parent, "arn")
                                .zip(scalar(evidence, source, "name"))
                                .zip(scalar(evidence, source, "arn"))
                                .is_some_and(|((arn, name), alias)| {
                                    alias == format!("{arn}:{name}")
                                })
                        })
            }
            _ => false,
        },
        "kms_key_arn" => {
            nodes[source].resource_type == "aws_kms_key" && matches(evidence, r, source, "arn")
        }
        "destination_config.on_failure.destination_arn" => {
            let kind = event_kind(nodes, refs, r.target, evidence);
            let kafka = matches!(kind, Some("aws_msk_cluster" | "aws_msk_serverless_cluster"))
                || kafka(value, mask);
            let stream = matches!(kind, Some("aws_dynamodb_table" | "aws_kinesis_stream"));
            matches(evidence, r, source, "arn")
                && match nodes[source].resource_type.as_str() {
                    "aws_sns_topic" | "aws_sqs_queue" => stream || kafka,
                    "aws_s3_bucket" => kafka,
                    _ => false,
                }
        }
        "amazon_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri"
        | "self_managed_kafka_event_source_config.schema_registry_config.schema_registry_uri" => {
            nodes[source].resource_type == "aws_glue_registry"
                && matches(evidence, r, source, "arn")
                && if r.attribute.starts_with("amazon_managed_") {
                    matches!(
                        event_kind(nodes, refs, r.target, evidence),
                        Some("aws_msk_cluster" | "aws_msk_serverless_cluster")
                    )
                } else {
                    kafka(value, mask)
                }
        }
        _ => false,
    }
}

fn access(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    if !super::service_evidence::eligible(nodes, r) {
        return false;
    }
    let (value, mask) = evidence[r.target];
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let Some(entries) = value["source_access_configuration"].as_array() else {
        return false;
    };
    let mask = &mask["source_access_configuration"];
    if entries.is_empty()
        || (!matches!(mask, Value::Null | Value::Bool(false))
            && mask.as_array().is_none_or(|m| m.len() != entries.len()))
    {
        return false;
    }
    let mut used = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for (i, entry) in entries.iter().enumerate() {
        let e = [(entry, mask.get(i).unwrap_or(&Value::Null))];
        let (Some(kind), Some(uri)) = (scalar(&e, 0, "type"), scalar(&e, 0, "uri")) else {
            return false;
        };
        if !pairs.insert((kind, uri)) {
            return false;
        }
        let (resource, field, prefix) = match kind {
            "BASIC_AUTH"
            | "SASL_SCRAM_256_AUTH"
            | "SASL_SCRAM_512_AUTH"
            | "CLIENT_CERTIFICATE_TLS_AUTH"
            | "SERVER_ROOT_CA_CERTIFICATE" => ("aws_secretsmanager_secret", "arn", ""),
            "VPC_SUBNET" => ("aws_subnet", "id", "subnet:"),
            "VPC_SECURITY_GROUP" => ("aws_security_group", "id", "security_group:"),
            _ => return false,
        };
        if !r.source_attributes.contains(field) {
            return false;
        }
        let mut found = r.sources.iter().copied().filter(|&s| {
            (nodes[s].resource_type == resource
                || (resource == "aws_secretsmanager_secret"
                    && nodes[s].resource_type == "aws_secretsmanager_secret_version"))
                && r.source_attributes_by_instance
                    .get(&s)
                    .is_some_and(|fields| fields.len() == 1 && fields.contains(field))
                && scalar(evidence, s, field).is_some_and(|id| uri.strip_prefix(prefix) == Some(id))
        });
        let Some(source) = found.next() else {
            return false;
        };
        if found.next().is_some() {
            return false;
        }
        used.insert(source);
    }
    used.len() == r.sources.len()
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != "aws_lambda_event_source_mapping" {
                return r.service_reference_verified;
            }
            if r.attribute == "source_access_configuration.uri" {
                return access(nodes, r, evidence);
            }
            if matches!(r.attribute.as_str(), "event_source_arn" | "function_name")
                || label(&nodes[r.target].resource_type, &r.attribute).is_some()
            {
                return base(nodes, refs, r, evidence);
            }
            r.service_reference_verified
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(verified) {
        r.service_reference_verified = verified;
    }
}
