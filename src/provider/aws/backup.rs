use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/backup.rs"]
mod tests;

pub(super) fn verified_resources(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
    };
    if !current(node)
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || reference.sources.is_empty()
        || reference.source_attributes.len() != 1
        || !reference.source_attributes.contains("arn")
    {
        return false;
    }
    let (value, unknown) = evidence[reference.target];
    if unknown == &Value::Bool(true) {
        return false;
    }
    for field in ["not_resources", "condition", "selection_tag"] {
        if !matches!(&unknown[field], Value::Null | Value::Bool(false))
            || !(value[field].is_null() || value[field].as_array().is_some_and(Vec::is_empty))
        {
            return false;
        }
    }
    let Some(values) = value["resources"].as_array() else {
        return false;
    };
    let mask = &unknown["resources"];
    if !matches!(mask, Value::Null | Value::Bool(false))
        && !mask.as_array().is_some_and(|masks| {
            masks.len() == values.len()
                && masks
                    .iter()
                    .all(|m| matches!(m, Value::Null | Value::Bool(false)))
        })
    {
        return false;
    }
    let Some(mut actual) = values.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
        return false;
    };
    if actual
        .iter()
        .any(|arn| !arn.starts_with("arn:") || arn.contains('*') || arn.contains('?'))
    {
        return false;
    }
    let mut expected = Vec::new();
    for &source in &reference.sources {
        let endpoint = &nodes[source];
        if !current(endpoint)
            || !super::service_references::cluster_binding_matches(
                node.provider_configuration.as_ref(),
                endpoint.provider_configuration.as_ref(),
                super::scope::scope(node)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                super::scope::scope(endpoint)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
            )
            || !matches!(
                endpoint.resource_type.as_str(),
                "aws_instance"
                    | "aws_ebs_volume"
                    | "aws_efs_file_system"
                    | "aws_db_instance"
                    | "aws_rds_cluster"
                    | "aws_dynamodb_table"
                    | "aws_s3_bucket"
            )
        {
            return false;
        }
        let (value, unknown) = evidence[source];
        let Some(arn) = super::service_references::scalar(value, unknown, &["arn"]) else {
            return false;
        };
        expected.push(arn);
    }
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}
