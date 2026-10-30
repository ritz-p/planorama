use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        ("aws_ecs_capacity_provider", "auto_scaling_group_provider.auto_scaling_group_arn") => {
            Some((
                "aws_autoscaling_group",
                "arn",
                false,
                "Capacity Auto Scaling group",
            ))
        }
        ("aws_ecs_capacity_provider", "cluster")
        | ("aws_ecs_cluster_capacity_providers", "cluster_name") => {
            Some(("aws_ecs_cluster", "name", false, "Configured cluster"))
        }
        ("aws_ecs_capacity_provider", "managed_instances_provider.infrastructure_role_arn") => {
            Some((
                "aws_iam_role",
                "arn",
                false,
                "Infrastructure role reference",
            ))
        }
        (
            "aws_ecs_capacity_provider",
            "managed_instances_provider.instance_launch_template.ec2_instance_profile_arn",
        ) => Some((
            "aws_iam_instance_profile",
            "arn",
            false,
            "Instance profile reference",
        )),
        (
            "aws_ecs_capacity_provider",
            "managed_instances_provider.instance_launch_template.network_configuration.subnets",
        ) => Some(("aws_subnet", "id", true, "Configured subnets")),
        (
            "aws_ecs_capacity_provider",
            "managed_instances_provider.instance_launch_template.network_configuration.security_groups",
        ) => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        (
            "aws_ecs_cluster_capacity_providers",
            "capacity_providers" | "default_capacity_provider_strategy.capacity_provider",
        )
        | ("aws_ecs_service", "capacity_provider_strategy.capacity_provider") => Some((
            "aws_ecs_capacity_provider",
            "name",
            true,
            "Configured capacity providers",
        )),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.3)
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&i| {
        let endpoint = &raw.nodes[i];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            matches!(
                endpoint.resource_type.as_str(),
                "aws_iam_role" | "aws_iam_instance_profile"
            ),
        )
    })
}

fn variant(value: &Value, mask: &Value, managed: bool) -> bool {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let (active, inactive) = if managed {
        ("managed_instances_provider", "auto_scaling_group_provider")
    } else {
        ("auto_scaling_group_provider", "managed_instances_provider")
    };
    let active_known = matches!(&mask[active], Value::Null | Value::Bool(false))
        || mask[active].as_array().is_some_and(|m| {
            m.len() == 1 && matches!(&m[0], Value::Null | Value::Bool(false) | Value::Object(_))
        });
    active_known
        && value[active]
            .as_array()
            .is_some_and(|v| v.len() == 1 && v[0].is_object())
        && (value[inactive].is_null() || value[inactive].as_array().is_some_and(Vec::is_empty))
        && (matches!(&mask[inactive], Value::Null | Value::Bool(false))
            || (value[inactive].as_array().is_some_and(Vec::is_empty)
                && mask[inactive].as_array().is_some_and(Vec::is_empty)))
        && if managed {
            super::streaming::values(value, mask, "cluster", false).is_some()
        } else {
            (value["cluster"].is_null() || value["cluster"] == "")
                && matches!(&mask["cluster"], Value::Null | Value::Bool(false))
        }
}

fn values<'a>(
    value: &'a Value,
    mask: &'a Value,
    path: &str,
    collection: bool,
) -> Option<BTreeSet<&'a str>> {
    if let Some(block) = path.strip_suffix(".capacity_provider") {
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        let entries = value[block].as_array()?;
        if entries.is_empty() {
            return None;
        }
        let masks = match &mask[block] {
            Value::Null | Value::Bool(false) => None,
            Value::Array(m) if m.len() == entries.len() => Some(m),
            _ => return None,
        };
        let mut result = BTreeSet::new();
        for (i, entry) in entries.iter().enumerate() {
            let item = super::streaming::values(
                entry,
                masks.map_or(&Value::Null, |m| &m[i]),
                "capacity_provider",
                false,
            )?;
            if !result.insert(*item.first()?) {
                return None;
            }
        }
        Some(result)
    } else {
        super::streaming::values(value, mask, path, collection)
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    for r in references {
        let node = &nodes[r.target];
        let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(field)
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if node.resource_type == "aws_ecs_capacity_provider"
            && !variant(
                value,
                mask,
                !r.attribute.starts_with("auto_scaling_group_provider."),
            )
        {
            continue;
        }
        if node.resource_type == "aws_ecs_capacity_provider" {
            let mut block = value;
            let mut valid = true;
            let parts: Vec<_> = r.attribute.split('.').collect();
            for field in &parts[..parts.len() - 1] {
                let Some(entries) = block[*field].as_array() else {
                    valid = false;
                    break;
                };
                let [entry] = entries.as_slice() else {
                    valid = false;
                    break;
                };
                block = entry;
            }
            if !valid {
                continue;
            }
        }
        let Some(actual) = values(value, mask, &r.attribute, collection) else {
            continue;
        };
        if kind == "aws_ecs_capacity_provider"
            && actual
                .iter()
                .any(|v| matches!(*v, "FARGATE" | "FARGATE_SPOT"))
        {
            continue;
        }
        let expected = r
            .sources
            .iter()
            .map(|&i| {
                let endpoint = &nodes[i];
                (current(endpoint)
                    && endpoint.resource_type == kind
                    && super::eventbridge::bindings(
                        node.provider_configuration.as_ref(),
                        endpoint.provider_configuration.as_ref(),
                        super::scope::scope(node)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(endpoint)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        matches!(kind, "aws_iam_role" | "aws_iam_instance_profile"),
                    ))
                .then(|| super::service_evidence::known(evidence, i, field))
                .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/ecs_capacity.rs"]
mod tests;
