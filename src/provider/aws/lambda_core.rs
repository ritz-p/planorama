use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_core.rs"]
mod tests;

const SUBNETS: &str = "configuration.vpc_egress_configuration.subnet_ids";
const GROUPS: &str = "configuration.vpc_egress_configuration.security_group_ids";
fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    if kind != "aws_lambdacore_network_connector" {
        return None;
    }
    match path {
        "operator_role" => Some(("aws_iam_role", "arn", false, "Connector operator role")),
        SUBNETS => Some(("aws_subnet", "id", true, "Egress subnets")),
        GROUPS => Some(("aws_security_group", "id", true, "Egress security groups")),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
}
pub(super) fn unique<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    found.next().is_none().then_some(r)
}
pub(super) fn parent(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    member: usize,
) -> Option<usize> {
    let r = unique(refs, member, "vpc_id")?;
    let [source] = r.sources.as_slice() else {
        return None;
    };
    (super::service_evidence::eligible(nodes, r)
        && r.source_attributes.len() == 1
        && r.source_attributes.contains("id")
        && nodes[*source].resource_type == "aws_vpc"
        && super::service_evidence::known(evidence, member, "vpc_id").is_some_and(|value| {
            super::service_evidence::known(evidence, *source, "id") == Some(value)
        }))
    .then_some(*source)
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let (kind, field, collection, _) = rule(&nodes[r.target].resource_type, &r.attribute)?;
            let proof = || {
                if unique(refs, r.target, &r.attribute).is_none()
                    || !super::service_evidence::eligible_with_global_role(
                        nodes,
                        r,
                        kind == "aws_iam_role",
                    )
                    || (!collection && r.sources.len() != 1)
                    || r.source_attributes.len() != 1
                    || !r.source_attributes.contains(field)
                {
                    return None;
                }
                let (value, mask) = evidence[r.target];
                if collection
                    && (value["configuration"]
                        .as_array()
                        .is_none_or(|a| a.len() != 1)
                        || value["configuration"][0]["vpc_egress_configuration"]
                            .as_array()
                            .is_none_or(|a| a.len() != 1))
                {
                    return None;
                }
                if collection
                    && super::streaming::values(
                        value,
                        mask,
                        "configuration.vpc_egress_configuration.associated_compute_resource_types",
                        true,
                    ) != Some(["MicroVm"].into())
                {
                    return None;
                }
                let actual = super::streaming::values(value, mask, &r.attribute, collection)?;
                let expected = r
                    .sources
                    .iter()
                    .map(|&s| {
                        (nodes[s].resource_type == kind)
                            .then(|| super::service_evidence::known(evidence, s, field))
                            .flatten()
                    })
                    .collect::<Option<BTreeSet<_>>>()?;
                if actual != expected || actual.len() != r.sources.len() {
                    return None;
                }
                Some(())
            };
            let verified = proof().is_some();
            let context = if verified && collection {
                r.sources
                    .iter()
                    .map(|&s| parent(nodes, refs, evidence, s))
                    .collect::<Option<BTreeSet<_>>>()
                    .filter(|parents| parents.len() == 1)
                    .map(|parents| parents.into_iter().collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Some((verified, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((verified, context)) = result {
            r.service_reference_verified = verified;
            r.endpoint_network_context = context;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&s| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            r.attribute == "operator_role",
        )
    })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    present_network(
        raw,
        graph,
        "aws_lambdacore_network_connector",
        (SUBNETS, GROUPS),
        bindings,
    );
}

pub(super) fn present_network(
    raw: &Input,
    graph: &mut Graph,
    kind: &str,
    paths: (&str, &str),
    compatible: fn(&Input, &AttributeReference) -> bool,
) {
    for (target, node) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == kind)
    {
        let proof = |path| {
            unique(&raw.attributes, target, path)
                .filter(|r| r.service_reference_verified && compatible(raw, r))
        };
        let Some((subnets, groups)) = proof(paths.0).zip(proof(paths.1)) else {
            continue;
        };
        let [parent] = subnets.endpoint_network_context.as_slice() else {
            continue;
        };
        if groups.endpoint_network_context != [*parent] {
            continue;
        }
        let vpc = &raw.nodes[*parent];
        if !super::service_references::cluster_binding_matches(
            node.provider_configuration.as_ref(),
            vpc.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            vpc.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        ) {
            continue;
        }
        let members: Vec<_> = subnets
            .sources
            .iter()
            .chain(&groups.sources)
            .copied()
            .collect();
        if !members.iter().all(|&s| {
            unique(&raw.attributes, s, "vpc_id").is_some_and(|r| super::apprunner::bindings(raw, r))
        }) {
            continue;
        }
        let evidence = members
            .into_iter()
            .flat_map(|s| {
                [
                    relationships::reference(graph, *parent, s),
                    relationships::reference(graph, s, target),
                ]
            })
            .collect();
        relationships::record(graph, *parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*parent, target))
        });
    }
}
