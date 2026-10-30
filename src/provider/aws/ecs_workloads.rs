use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

fn rule(
    resource: &str,
    attribute: &str,
    output: &str,
) -> Option<(&'static str, bool, &'static str)> {
    match (resource, attribute, output) {
        ("aws_ecs_daemon", "cluster_arn", "arn") => {
            Some(("aws_ecs_cluster", false, "Cluster membership"))
        }
        ("aws_ecs_daemon", "capacity_provider_arns", "arn") => Some((
            "aws_ecs_capacity_provider",
            true,
            "Configured capacity providers",
        )),
        ("aws_ecs_daemon", "daemon_task_definition_arn", "arn") => Some((
            "aws_ecs_daemon_task_definition",
            false,
            "Daemon task revision",
        )),
        (
            "aws_ecs_daemon_task_definition"
            | "aws_ecs_task_definition"
            | "aws_ecs_express_gateway_service",
            role @ ("execution_role_arn" | "task_role_arn"),
            "arn",
        ) => Some((
            "aws_iam_role",
            false,
            if role == "execution_role_arn" {
                "Execution role reference"
            } else {
                "Task role reference"
            },
        )),
        ("aws_ecs_express_gateway_service", "infrastructure_role_arn", "arn") => {
            Some(("aws_iam_role", false, "Infrastructure role reference"))
        }
        ("aws_ecs_express_gateway_service" | "aws_ecs_task_set", "cluster", "name" | "arn") => {
            Some(("aws_ecs_cluster", false, "Cluster membership"))
        }
        ("aws_ecs_task_set", "service", "name" | "arn") => {
            Some(("aws_ecs_service", false, "Task set service"))
        }
        ("aws_ecs_service" | "aws_ecs_task_set", "task_definition", "arn") => {
            Some(("aws_ecs_task_definition", false, "Task definition revision"))
        }
        ("aws_ecs_service" | "aws_ecs_task_set", "service_registries.registry_arn", "arn") => {
            Some(("aws_service_discovery_service", false, "Service registry"))
        }
        (
            "aws_ecs_express_gateway_service" | "aws_ecs_task_set",
            "network_configuration.subnets",
            "id",
        ) => Some(("aws_subnet", true, "Configured subnets")),
        (
            "aws_ecs_express_gateway_service" | "aws_ecs_task_set",
            "network_configuration.security_groups",
            "id",
        ) => Some(("aws_security_group", true, "Configured security groups")),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    ["arn", "name", "id"]
        .into_iter()
        .find_map(|field| rule(resource, attribute, field).map(|r| r.2))
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    super::ecs_capacity::bindings(raw, r)
}

fn revision(arn: &str, daemon: bool) -> bool {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.len() != 6
        || parts[0] != "arn"
        || parts[1].is_empty()
        || parts[2] != "ecs"
        || parts[3].is_empty()
        || parts[4].len() != 12
        || !parts[4].bytes().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    let prefix = if daemon {
        "daemon-task-definition/"
    } else {
        "task-definition/"
    };
    parts[5]
        .strip_prefix(prefix)
        .and_then(|s| s.rsplit_once(':'))
        .is_some_and(|(family, revision)| {
            !family.is_empty()
                && !family.contains([':', '/'])
                && revision.bytes().all(|c| c.is_ascii_digit())
                && revision.parse::<u64>().is_ok_and(|n| n > 0)
        })
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    for r in refs.iter_mut() {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let field = r.source_attributes.iter().next().unwrap();
        let Some((kind, collection, _)) = rule(&node.resource_type, &r.attribute, field) else {
            continue;
        };
        if !collection && r.sources.len() != 1 {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if let Some((block, _)) = r.attribute.split_once('.') {
            if !value[block]
                .as_array()
                .is_some_and(|v| v.len() == 1 && v[0].is_object())
            {
                continue;
            }
        }
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
            continue;
        };
        if matches!(
            kind,
            "aws_ecs_task_definition" | "aws_ecs_daemon_task_definition"
        ) && !actual
            .iter()
            .all(|v| revision(v, kind == "aws_ecs_daemon_task_definition"))
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
                        kind == "aws_iam_role",
                    ))
                .then(|| super::service_evidence::known(evidence, i, field))
                .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
    let cluster = |index| {
        let mut candidates = refs.iter().filter(|r| {
            r.target == index && r.attribute == "cluster" && r.service_reference_verified
        });
        let r = candidates.next()?;
        (candidates.next().is_none() && r.sources.len() == 1).then_some(r.sources[0])
    };
    let valid: BTreeSet<_> = refs
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            if nodes[r.target].resource_type != "aws_ecs_task_set"
                || r.attribute != "service"
                || !r.service_reference_verified
            {
                return None;
            }
            cluster(r.target)
                .zip(cluster(r.sources[0]))
                .filter(|(a, b)| a == b)
                .map(|_| i)
        })
        .collect();
    for (i, r) in refs.iter_mut().enumerate() {
        if nodes[r.target].resource_type == "aws_ecs_task_set" && r.attribute == "service" {
            r.service_reference_verified &= valid.contains(&i);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/ecs_workloads.rs"]
mod tests;
