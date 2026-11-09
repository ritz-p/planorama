use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/eks_auxiliary.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static str, &'static str)> {
    match (kind, field) {
        (
            "aws_eks_access_policy_association"
            | "aws_eks_identity_provider_config"
            | "aws_eks_pod_identity_association"
            | "aws_eks_capability",
            "cluster_name",
        ) => Some(("aws_eks_cluster", "name")),
        ("aws_eks_pod_identity_association", "role_arn" | "target_role_arn")
        | ("aws_eks_capability", "role_arn")
        | ("aws_eks_addon", "pod_identity_association.role_arn") => Some(("aws_iam_role", "arn")),
        ("aws_eks_access_policy_association", "principal_arn") => {
            Some(("aws_eks_access_entry", "principal_arn"))
        }
        ("aws_eks_node_group", "launch_template.id") => Some(("aws_launch_template", "id")),
        ("aws_eks_node_group", "launch_template.name") => Some(("aws_launch_template", "name")),
        _ => None,
    }
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "cluster_name" => "Configured EKS cluster",
        "principal_arn" => "Cluster access entry",
        "launch_template.id" | "launch_template.name" => "Versioned launch template",
        _ => "Pod or capability role",
    })
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            let Some((kind, field)) = rule(&nodes[r.target].resource_type, &r.attribute) else {
                return r.service_reference_verified;
            };
            if !super::service_evidence::eligible_with_global_role(nodes, r, kind == "aws_iam_role")
                || r.source_attributes.len() != 1
                || !r.source_attributes.contains(field)
            {
                return false;
            }
            let expected = r
                .sources
                .iter()
                .map(|&s| {
                    (nodes[s].resource_type == kind)
                        .then(|| super::service_evidence::known(evidence, s, field))
                        .flatten()
                })
                .collect::<Option<BTreeSet<_>>>();
            let (value, unknown) = evidence[r.target];
            let actual = if r.attribute == "pod_identity_association.role_arn" {
                if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                    || !matches!(
                        &unknown["pod_identity_association"],
                        Value::Null | Value::Bool(false)
                    )
                {
                    return false;
                }
                value["pod_identity_association"]
                    .as_array()
                    .and_then(|rows| {
                        rows.iter()
                            .map(|row| row["role_arn"].as_str().filter(|s| !s.is_empty()))
                            .collect::<Option<BTreeSet<_>>>()
                    })
            } else {
                if r.sources.len() != 1 {
                    return false;
                }
                super::streaming::values(value, unknown, &r.attribute, false)
            };
            actual.is_some()
                && actual == expected
                && expected.is_some_and(|v| v.len() == r.sources.len())
        })
        .collect();
    let source = |target, field| {
        let mut candidates = refs
            .iter()
            .enumerate()
            .filter(|(_, r)| r.target == target && r.attribute == field);
        let (i, r) = candidates.next()?;
        if candidates.next().is_some() || !verified[i] {
            return None;
        }
        match r.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    let result: Vec<_> = refs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            if !verified[i] {
                return false;
            }
            if nodes[r.target].resource_type == "aws_eks_access_policy_association"
                && r.attribute == "principal_arn"
            {
                return source(r.target, "cluster_name")
                    .is_some_and(|cluster| source(r.sources[0], "cluster_name") == Some(cluster));
            }
            if nodes[r.target].resource_type == "aws_eks_node_group"
                && r.attribute.starts_with("launch_template.")
            {
                if refs
                    .iter()
                    .filter(|v| {
                        v.target == r.target
                            && matches!(
                                v.attribute.as_str(),
                                "launch_template.id" | "launch_template.name"
                            )
                    })
                    .count()
                    != 1
                {
                    return false;
                }
                let (value, unknown) = evidence[r.target];
                let Some(version) =
                    super::streaming::values(value, unknown, "launch_template.version", false)
                        .and_then(|s| s.first().copied())
                else {
                    return false;
                };
                let Ok(version) = version.parse::<u64>() else {
                    return false;
                };
                if version == 0 {
                    return false;
                }
                let mut versions = refs
                    .iter()
                    .filter(|v| v.target == r.target && v.attribute == "launch_template.version");
                let Some(v) = versions.next() else {
                    return false;
                };
                if versions.next().is_some()
                    || v.sources != r.sources
                    || v.source_attributes.len() != 1
                    || !super::service_evidence::eligible(nodes, v)
                {
                    return false;
                }
                let field = v.source_attributes.first().unwrap();
                if !matches!(field.as_str(), "latest_version" | "default_version") {
                    return false;
                }
                let (value, unknown) = evidence[r.sources[0]];
                return matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                    && matches!(&unknown[field], Value::Null | Value::Bool(false))
                    && value[field].as_u64() == Some(version);
            }
            true
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(result) {
        if rule(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = valid;
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role",
        )
    })
}
