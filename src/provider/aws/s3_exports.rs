use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_exports.rs"]
mod tests;

pub(super) fn kms_arn(kind: &str, arn: &str) -> bool {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    matches!(parts.as_slice(), ["arn", partition, "kms", region, account, resource]
    if !partition.is_empty() && !region.is_empty() && !account.is_empty()
    && match kind {
        "aws_kms_key" => resource.strip_prefix("key/").is_some_and(|id| !id.is_empty()),
        "aws_kms_alias" => resource.strip_prefix("alias/").is_some_and(|id| !id.is_empty()),
        _ => false,
    })
}

type ReferenceRule = (&'static [&'static str], &'static [&'static str], bool);

fn rule(resource: &str, path: &str) -> Option<ReferenceRule> {
    match (resource, path) {
        (
            "aws_s3_bucket_analytics_configuration"
            | "aws_s3_bucket_logging"
            | "aws_s3_bucket_metadata_configuration",
            "bucket",
        )
        | ("aws_s3_bucket_logging", "target_bucket") => {
            Some((&["aws_s3_bucket"], &["id", "bucket"], false))
        }
        ("aws_s3_bucket_inventory", "bucket") => Some((
            &["aws_s3_bucket", "aws_s3_directory_bucket"],
            &["id", "bucket"],
            false,
        )),
        (
            "aws_s3_bucket_analytics_configuration",
            "storage_class_analysis.data_export.destination.s3_bucket_destination.bucket_arn",
        )
        | ("aws_s3_bucket_inventory", "destination.bucket.bucket_arn")
        | (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.data_export.s3_bucket_destination.arn"
            | "storage_lens_configuration.expanded_prefixes_data_export.s3_bucket_destination.arn",
        ) => Some((&["aws_s3_bucket"], &["arn"], false)),
        ("aws_s3_bucket_inventory", "destination.bucket.encryption.sse_kms.key_id")
        | (
            "aws_s3_bucket_metadata_configuration",
            "metadata_configuration.inventory_table_configuration.encryption_configuration.kms_key_arn"
            | "metadata_configuration.journal_table_configuration.encryption_configuration.kms_key_arn",
        )
        | (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.data_export.s3_bucket_destination.encryption.sse_kms.key_id",
        ) => Some((&["aws_kms_key"], &["arn"], false)),
        ("aws_s3_bucket_replication_configuration", "role") => {
            Some((&["aws_iam_role"], &["arn"], false))
        }
        ("aws_s3control_storage_lens_configuration", "storage_lens_configuration.aws_org.arn") => {
            Some((&["aws_organizations_organization"], &["arn"], false))
        }
        (
            "aws_s3control_storage_lens_configuration",
            "storage_lens_configuration.include.buckets"
            | "storage_lens_configuration.exclude.buckets",
        ) => Some((&["aws_s3_bucket"], &["arn"], true)),
        _ => None,
    }
}

pub(super) fn label(resource: &str, path: &str) -> Option<&'static str> {
    rule(resource, path).map(|(_, fields, collection)| {
        if collection {
            "Bucket selector references"
        } else if fields.contains(&"bucket") {
            "Configured bucket reference"
        } else {
            "Configured export reference"
        }
    })
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn optional_scalar<'a>(
    value: &'a Value,
    unknown: &Value,
    path: &[&str],
) -> Option<Option<&'a str>> {
    if unknown == &Value::Bool(true) {
        return None;
    }
    if let Some(items) = value.as_array() {
        let [item] = items.as_slice() else {
            return None;
        };
        return optional_scalar(item, &unknown[0], path);
    }
    if let Some((field, rest)) = path.split_first() {
        return optional_scalar(&value[*field], &unknown[*field], rest);
    }
    if !matches!(unknown, Value::Null | Value::Bool(false)) {
        return None;
    }
    if value.is_null() {
        Some(None)
    } else {
        value.as_str().filter(|s| !s.is_empty()).map(Some)
    }
}

fn singleton_path(value: &Value, path: &str) -> bool {
    let mut value = value;
    let parts: Vec<_> = path.split('.').collect();
    for field in &parts[..parts.len() - 1] {
        let Some(items) = value[*field].as_array() else {
            return false;
        };
        let [item] = items.as_slice() else {
            return false;
        };
        value = item;
    }
    true
}

fn absent(value: &Value, unknown: &Value, path: &[&str]) -> bool {
    if unknown == &Value::Bool(true) {
        return false;
    }
    if path.is_empty() {
        return matches!(unknown, Value::Null | Value::Bool(false))
            && (value.is_null() || value.as_array().is_some_and(Vec::is_empty));
    }
    if let Some(items) = value.as_array() {
        let [item] = items.as_slice() else {
            return false;
        };
        return absent(item, &unknown[0], path);
    }
    absent(&value[path[0]], &unknown[path[0]], &path[1..])
}

fn verify(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let Some((kinds, fields, collection)) = rule(&node.resource_type, &reference.attribute) else {
        return false;
    };
    if !current(node)
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || reference.sources.is_empty()
        || (!collection && reference.sources.len() != 1)
        || reference.source_attributes.len() != 1
        || !fields
            .iter()
            .any(|field| reference.source_attributes.contains(*field))
    {
        return false;
    }
    let (value, unknown) = evidence[reference.target];
    if reference.attribute.ends_with("sse_kms.key_id") {
        let other = reference.attribute.replace("sse_kms.key_id", "sse_s3");
        if !absent(value, unknown, &other.split('.').collect::<Vec<_>>()) {
            return false;
        }
    }
    if !singleton_path(value, &reference.attribute) {
        return false;
    }
    let Some(actual) = super::streaming::values(value, unknown, &reference.attribute, collection)
    else {
        return false;
    };
    if node.resource_type == "aws_s3_bucket_metadata_configuration"
        && reference.attribute.ends_with("kms_key_arn")
    {
        let mode = reference.attribute.replace("kms_key_arn", "sse_algorithm");
        if super::service_references::scalar(value, unknown, &mode.split('.').collect::<Vec<_>>())
            != Some("aws:kms")
        {
            return false;
        }
    }
    let mut expected = BTreeSet::new();
    for &source in &reference.sources {
        let target = &nodes[source];
        if !current(target) || !kinds.contains(&target.resource_type.as_str()) {
            return false;
        }
        let (target_value, target_unknown) = evidence[source];
        let field = reference.source_attributes.iter().next().unwrap();
        let Some(identifier) =
            super::service_references::scalar(target_value, target_unknown, &[field])
        else {
            return false;
        };
        if target.resource_type == "aws_kms_key" && !kms_arn(&target.resource_type, identifier) {
            return false;
        }
        if !expected.insert(identifier) {
            return false;
        }
        if matches!(
            node.resource_type.as_str(),
            "aws_s3_bucket_analytics_configuration" | "aws_s3_bucket_inventory"
        ) && reference.attribute.ends_with("bucket_arn")
        {
            let account_path = reference.attribute.replace(
                "bucket_arn",
                if node.resource_type == "aws_s3_bucket_analytics_configuration" {
                    "bucket_account_id"
                } else {
                    "account_id"
                },
            );
            let path = account_path.split('.').collect::<Vec<_>>();
            let Some(account) = optional_scalar(value, unknown, &path) else {
                return false;
            };
            if let Some(account) = account {
                if super::scope::scope(target)
                    .and_then(|scope| scope.account)
                    .as_ref()
                    .map(|account| account.as_str())
                    != Some(account)
                {
                    return false;
                }
            }
        }
        if node.resource_type == "aws_s3control_storage_lens_configuration"
            && reference.attribute.ends_with("s3_bucket_destination.arn")
        {
            let path = reference.attribute.strip_suffix("arn").unwrap().to_owned() + "account_id";
            let Some(account) = super::service_references::scalar(
                value,
                unknown,
                &path.split('.').collect::<Vec<_>>(),
            ) else {
                return false;
            };
            if super::scope::scope(target)
                .and_then(|scope| scope.account)
                .as_ref()
                .map(|account| account.as_str())
                != Some(account)
            {
                return false;
            }
        }
    }
    actual == expected
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for reference in references {
        if label(&nodes[reference.target].resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified = verify(nodes, reference, evidence);
        }
    }
}
