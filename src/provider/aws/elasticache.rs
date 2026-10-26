use crate::model::{
    Action, AttributeReference, EntityMode, ProviderConfiguration, RegionScope, TerraformEntity,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/elasticache.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_elasticache_global_replication_group", "primary_replication_group_id") => {
            Some("Configured primary replication group")
        }
        ("aws_elasticache_replication_group", "global_replication_group_id") => {
            Some("Global datastore membership")
        }
        ("aws_elasticache_cluster", "replication_group_id") => {
            Some("Cache replication group membership")
        }
        (
            "aws_elasticache_cluster" | "aws_elasticache_replication_group",
            "parameter_group_name",
        ) => Some("Cache parameter group"),
        ("aws_elasticache_replication_group", "user_group_ids")
        | (
            "aws_elasticache_serverless_cache" | "aws_elasticache_user_group_association",
            "user_group_id",
        ) => Some("Cache user group"),
        ("aws_elasticache_user_group", "user_ids")
        | ("aws_elasticache_user_group_association", "user_id") => Some("Cache user membership"),
        _ => None,
    }
}

fn fields(resource: &str, attribute: &str, endpoint: &str) -> &'static [&'static str] {
    match (resource, attribute, endpoint) {
        (
            "aws_elasticache_global_replication_group",
            "primary_replication_group_id",
            "aws_elasticache_replication_group",
        )
        | (
            "aws_elasticache_cluster",
            "replication_group_id",
            "aws_elasticache_replication_group",
        ) => &["id", "replication_group_id"],
        (
            "aws_elasticache_replication_group",
            "global_replication_group_id",
            "aws_elasticache_global_replication_group",
        ) => &["id", "global_replication_group_id"],
        (
            "aws_elasticache_cluster" | "aws_elasticache_replication_group",
            "parameter_group_name",
            "aws_elasticache_parameter_group",
        ) => &["id", "name"],
        ("aws_elasticache_replication_group", "user_group_ids", "aws_elasticache_user_group")
        | (
            "aws_elasticache_serverless_cache" | "aws_elasticache_user_group_association",
            "user_group_id",
            "aws_elasticache_user_group",
        ) => &["id", "user_group_id"],
        ("aws_elasticache_user_group", "user_ids", "aws_elasticache_user")
        | ("aws_elasticache_user_group_association", "user_id", "aws_elasticache_user") => {
            &["id", "user_id"]
        }
        _ => &[],
    }
}

fn regions_match(
    attribute: &str,
    consumer: Option<&RegionScope>,
    endpoint: Option<&RegionScope>,
) -> bool {
    match attribute {
        "primary_replication_group_id" => endpoint.is_some(),
        "global_replication_group_id" => consumer.is_some(),
        _ => consumer.is_some() && consumer == endpoint,
    }
}

pub(super) fn bindings(
    attribute: &str,
    consumer: Option<&ProviderConfiguration>,
    endpoint: Option<&ProviderConfiguration>,
    consumer_region: Option<&RegionScope>,
    endpoint_region: Option<&RegionScope>,
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
        && regions_match(attribute, consumer_region, endpoint_region)
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    if !current(node)
        || !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || reference.sources.is_empty()
        || reference.source_attributes.len() != 1
    {
        return false;
    }
    let field = reference
        .source_attributes
        .iter()
        .next()
        .expect("one source attribute");
    let (value, unknown) = evidence[reference.target];
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let actual: Vec<&str> = if matches!(reference.attribute.as_str(), "user_ids" | "user_group_ids")
    {
        let Some(values) = value[&reference.attribute].as_array() else {
            return false;
        };
        let mask = &unknown[&reference.attribute];
        if !matches!(mask, Value::Null | Value::Bool(false))
            && !mask.as_array().is_some_and(|m| {
                m.len() == values.len()
                    && m.iter()
                        .all(|m| matches!(m, Value::Null | Value::Bool(false)))
            })
        {
            return false;
        }
        let Some(values) = values
            .iter()
            .map(|v| v.as_str().filter(|s| !s.is_empty()))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        values
    } else {
        let Some(value) =
            super::service_evidence::known(evidence, reference.target, &reference.attribute)
        else {
            return false;
        };
        if reference.sources.len() != 1 {
            return false;
        }
        vec![value]
    };
    let mut expected = BTreeSet::new();
    for &source in &reference.sources {
        let endpoint = &nodes[source];
        let left = super::scope::scope(node);
        let right = super::scope::scope(endpoint);
        if !current(endpoint)
            || !fields(
                &node.resource_type,
                &reference.attribute,
                &endpoint.resource_type,
            )
            .contains(&field.as_str())
            || !regions_match(
                &reference.attribute,
                left.as_ref().and_then(|s| s.region.as_ref()),
                right.as_ref().and_then(|s| s.region.as_ref()),
            )
        {
            return false;
        }
        let Some(value) = super::service_evidence::known(evidence, source, field) else {
            return false;
        };
        if !expected.insert(value) {
            return false;
        }
    }
    actual.len() == expected.len() && actual.into_iter().collect::<BTreeSet<_>>() == expected
}
