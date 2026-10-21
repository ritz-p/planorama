use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn endpoint_type(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_db_proxy", "vpc_subnet_ids") => Some("aws_subnet"),
        ("aws_db_proxy", "vpc_security_group_ids") => Some("aws_security_group"),
        ("aws_eks_node_group" | "aws_eks_fargate_profile", "subnet_ids") => Some("aws_subnet"),
        _ => None,
    }
}

pub(super) fn verified(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let Some(kind) = endpoint_type(&node.resource_type, &reference.attribute) else {
        return false;
    };
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
        || !reference.ids_only
        || reference.sources.is_empty()
    {
        return false;
    }
    let (value, unknown) = evidence[reference.target];
    if unknown == &Value::Bool(true) {
        return false;
    }
    let mask = &unknown[&reference.attribute];
    let Some(values) = value[&reference.attribute].as_array() else {
        return false;
    };
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
    let mut expected = Vec::new();
    for &source in &reference.sources {
        let endpoint = &nodes[source];
        if !current(endpoint)
            || endpoint.resource_type != kind
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
        {
            return false;
        }
        let (value, unknown) = evidence[source];
        let Some(id) = super::service_references::scalar(value, unknown, &["id"]) else {
            return false;
        };
        expected.push(id);
    }
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}
