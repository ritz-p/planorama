use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/iam.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/identity_providers.rs"]
mod identity_provider_tests;

fn rule(resource: &str, attribute: &str) -> Option<(&'static str, &'static str, bool)> {
    match (resource, attribute) {
        ("aws_iam_group_membership", "group")
        | ("aws_iam_group_policy" | "aws_iam_group_policy_attachment", "group") => {
            Some(("aws_iam_group", "name", false))
        }
        ("aws_iam_user_group_membership", "user")
        | ("aws_iam_user_policy" | "aws_iam_user_policy_attachment", "user") => {
            Some(("aws_iam_user", "name", false))
        }
        ("aws_iam_group_membership", "users") => Some(("aws_iam_user", "name", true)),
        ("aws_iam_user_group_membership", "groups") => Some(("aws_iam_group", "name", true)),
        ("aws_iam_user_policy_attachment" | "aws_iam_group_policy_attachment", "policy_arn") => {
            Some(("aws_iam_policy", "arn", false))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute)?;
    Some(match attribute {
        "users" => "Member users",
        "groups" => "Member groups",
        "policy_arn" => "Direct policy",
        "user" => "User",
        _ => "Group",
    })
}

pub(super) fn verified(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[reference.target];
    let Some((resource_type, field, collection)) = rule(&node.resource_type, &reference.attribute)
    else {
        return false;
    };
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
        || !reference.source_attributes.contains(field)
        || (!collection && reference.sources.len() != 1)
    {
        return false;
    }
    let mut expected = BTreeSet::new();
    for &source in &reference.sources {
        let endpoint = &nodes[source];
        if !current(endpoint)
            || endpoint.resource_type != resource_type
            || source == reference.target
        {
            return false;
        }
        let (value, unknown) = evidence[source];
        let Some(name) = super::service_references::scalar(value, unknown, &[field]) else {
            return false;
        };
        if !expected.insert(name) {
            return false;
        }
    }
    let (value, unknown) = evidence[reference.target];
    let attribute = reference.attribute.as_str();
    if !collection {
        return super::service_references::scalar(value, unknown, &[attribute])
            .is_some_and(|value| expected.contains(value));
    }
    if unknown == &Value::Bool(true) || unknown[attribute] == Value::Bool(true) {
        return false;
    }
    let Some(values) = value[attribute].as_array() else {
        return false;
    };
    if unknown[attribute]
        .as_array()
        .is_some_and(|masks| masks.len() != values.len())
    {
        return false;
    }
    let actual: Option<BTreeSet<_>> = values
        .iter()
        .enumerate()
        .map(|(i, value)| super::service_references::scalar(value, &unknown[attribute][i], &[]))
        .collect();
    actual.is_some_and(|actual| actual == expected && actual.len() == values.len())
}
