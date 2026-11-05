use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/scraper.rs"]
mod tests;

fn rule(path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match path {
        "source.eks.cluster_arn" => Some(("aws_eks_cluster", "arn", false, "Scraper EKS source")),
        "source.eks.subnet_ids" | "source.vpc.subnet_ids" => {
            Some(("aws_subnet", "id", true, "Scraper source subnets"))
        }
        "source.eks.security_group_ids" | "source.vpc.security_group_ids" => Some((
            "aws_security_group",
            "id",
            true,
            "Scraper source security groups",
        )),
        "destination.amp.workspace_arn" => Some((
            "aws_prometheus_workspace",
            "arn",
            false,
            "Scraper destination workspace",
        )),
        "exporter.opensearch.domain_arn" => Some((
            "aws_opensearch_domain",
            "arn",
            false,
            "Scraper OpenSearch exporter",
        )),
        "role_configuration.source_role_arn" | "role_configuration.target_role_arn" => {
            Some(("aws_iam_role", "arn", false, "Configured scraper role"))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, path: &str) -> Option<&'static str> {
    (resource == "aws_prometheus_scraper")
        .then(|| rule(path).map(|r| r.3))
        .flatten()
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

fn variant<'a>(value: &Value, mask: &Value, block: &str, options: &'a [&str]) -> Option<&'a str> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let [item] = value[block].as_array()?.as_slice() else {
        return None;
    };
    let block_mask = &mask[block];
    if !matches!(block_mask, Value::Null | Value::Bool(false))
        && block_mask.as_array().is_none_or(|a| a.len() != 1)
    {
        return None;
    }
    let mask = &block_mask[0];
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let mut selected = None;
    for &option in options {
        let child = &item[option];
        let child_mask = &mask[option];
        if child.is_null() || child.as_array().is_some_and(Vec::is_empty) {
            if !matches!(child_mask, Value::Null | Value::Bool(false)) {
                return None;
            }
        } else {
            if child.as_array().is_none_or(|a| a.len() != 1) || selected.is_some() {
                return None;
            }
            if !matches!(child_mask, Value::Null | Value::Bool(false))
                && child_mask.as_array().is_none_or(|masks| {
                    masks.len() != 1
                        || !matches!(
                            masks[0],
                            Value::Null | Value::Bool(false) | Value::Object(_)
                        )
                })
            {
                return None;
            }
            selected = Some(option);
        }
    }
    selected
}

fn cross_path(path: &str) -> bool {
    matches!(
        path,
        "destination.amp.workspace_arn" | "role_configuration.target_role_arn"
    )
}

fn compatible(node: &TerraformEntity, endpoint: &TerraformEntity, path: &str) -> bool {
    let left = node.provider_configuration.as_ref();
    let right = endpoint.provider_configuration.as_ref();
    let a = super::scope::scope(node);
    let b = super::scope::scope(endpoint);
    super::eventbridge::bindings(
        left,
        right,
        a.as_ref().and_then(|s| s.region.as_ref()),
        b.as_ref().and_then(|s| s.region.as_ref()),
        endpoint.resource_type == "aws_iam_role",
    ) || (cross_path(path)
        && a.as_ref().and_then(|s| s.region.as_ref()) == left.and_then(|c| c.region.as_ref())
        && (endpoint.resource_type == "aws_iam_role"
            || b.as_ref().and_then(|s| s.region.as_ref()) == right.and_then(|c| c.region.as_ref()))
        && left.zip(right).is_some_and(|(a, b)| {
            !a.account_conflict
                && !b.account_conflict
                && a.account.is_some()
                && b.account.is_some()
                && a.region.is_some()
                && a.region == b.region
        }))
}

fn basic(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    let node = &nodes[r.target];
    let Some((kind, field, collection, _)) = rule(&r.attribute) else {
        return false;
    };
    if !current(node)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(field)
    {
        return false;
    }
    let (value, mask) = evidence[r.target];
    let Some(source) = variant(value, mask, "source", &["eks", "vpc"]) else {
        return false;
    };
    let Some(destination) = variant(value, mask, "destination", &["amp", "cloudwatch"]) else {
        return false;
    };
    if (r.attribute.starts_with("source.")
        && !r.attribute.starts_with(&format!("source.{source}.")))
        || (r.attribute.starts_with("destination.") && destination != "amp")
    {
        return false;
    }
    if r.attribute.starts_with("exporter.")
        && variant(value, mask, "exporter", &["opensearch"]) != Some("opensearch")
    {
        return false;
    }
    if r.attribute.starts_with("role_configuration.")
        && value["role_configuration"]
            .as_array()
            .is_none_or(|a| a.len() != 1)
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
        return false;
    };
    if r.attribute.ends_with("subnet_ids") && actual.len() < 2 {
        return false;
    }
    let expected = r
        .sources
        .iter()
        .map(|&index| {
            let endpoint = &nodes[index];
            (current(endpoint)
                && endpoint.resource_type == kind
                && compatible(node, endpoint, &r.attribute))
            .then(|| super::service_evidence::known(evidence, index, field))
            .flatten()
        })
        .collect::<Option<BTreeSet<_>>>();
    expected.is_some_and(|expected| expected.len() == r.sources.len() && expected == actual)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut proofs = Vec::new();
    for (index, r) in refs
        .iter()
        .enumerate()
        .filter(|(_, r)| label(&nodes[r.target].resource_type, &r.attribute).is_some())
    {
        let mut valid = basic(nodes, r, evidence);
        if valid && cross_path(&r.attribute) {
            let target_account = nodes[r.sources[0]]
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref());
            let own_account = nodes[r.target]
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref());
            if target_account != own_account {
                let endpoint = |path| {
                    let mut candidates = refs
                        .iter()
                        .filter(|p| p.target == r.target && p.attribute == path);
                    let p = candidates.next()?;
                    (candidates.next().is_none() && basic(nodes, p, evidence)).then(|| p.sources[0])
                };
                valid = endpoint("destination.amp.workspace_arn")
                    .zip(endpoint("role_configuration.target_role_arn"))
                    .is_some_and(|(workspace, role)| {
                        endpoint("role_configuration.source_role_arn").is_some()
                            && nodes[workspace]
                                .provider_configuration
                                .as_ref()
                                .and_then(|c| c.account.as_ref())
                                == nodes[role]
                                    .provider_configuration
                                    .as_ref()
                                    .and_then(|c| c.account.as_ref())
                    });
            }
        }
        proofs.push((index, valid));
    }
    for (index, valid) in proofs {
        refs[index].service_reference_verified = valid;
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&source| {
        let node = &raw.nodes[r.target];
        let target = &raw.nodes[source];
        let left = node.provider_configuration.as_ref();
        let right = target.provider_configuration.as_ref();
        super::eventbridge::bindings(
            left,
            right,
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            target.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            target.resource_type == "aws_iam_role",
        ) || (cross_path(&r.attribute)
            && node.entity.scope.as_ref().and_then(|s| s.region.as_ref())
                == left.and_then(|c| c.region.as_ref())
            && (target.resource_type == "aws_iam_role"
                || target.entity.scope.as_ref().and_then(|s| s.region.as_ref())
                    == right.and_then(|c| c.region.as_ref()))
            && left.zip(right).is_some_and(|(a, b)| {
                !a.account_conflict
                    && !b.account_conflict
                    && a.account.is_some()
                    && b.account.is_some()
                    && a.region.is_some()
                    && a.region == b.region
            }))
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, _) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_prometheus_scraper")
    {
        let proof = |path: &str| {
            raw.attributes.iter().find(|r| {
                r.target == target
                    && r.attribute == path
                    && r.service_reference_verified
                    && bindings(raw, r)
            })
        };
        for variant in ["eks", "vpc"] {
            let Some((subnets, groups)) = proof(&format!("source.{variant}.subnet_ids"))
                .zip(proof(&format!("source.{variant}.security_group_ids")))
            else {
                continue;
            };
            let members = subnets
                .sources
                .iter()
                .chain(&groups.sources)
                .copied()
                .collect::<Vec<_>>();
            let Some(parents) = members
                .iter()
                .map(|&i| super::streaming::vpc(raw, i))
                .collect::<Option<BTreeSet<_>>>()
            else {
                continue;
            };
            if parents.len() != 1 {
                continue;
            }
            let parent = *parents.first().expect("one VPC");
            if variant == "eks" {
                let Some(cluster) = proof("source.eks.cluster_arn") else {
                    continue;
                };
                let Some(network) = raw.attributes.iter().find(|r| {
                    r.target == cluster.sources[0]
                        && r.attribute == "vpc_config.subnet_ids"
                        && r.complete
                        && r.instances_explicit
                        && r.collection_ids_complete
                        && r.issues.is_empty()
                }) else {
                    continue;
                };
                if network.sources.is_empty()
                    || !super::streaming::bindings(raw, network)
                    || network.sources.iter().any(|&i| {
                        raw.nodes[i].resource_type != "aws_subnet"
                            || super::streaming::vpc(raw, i) != Some(parent)
                    })
                {
                    continue;
                }
            }
            let evidence = members
                .iter()
                .flat_map(|&member| {
                    [
                        relationships::reference(graph, parent, member),
                        relationships::reference(graph, member, target),
                    ]
                })
                .collect();
            relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
            graph.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((parent, target))
            });
        }
    }
}
