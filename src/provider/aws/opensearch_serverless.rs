use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/opensearch_serverless.rs"]
mod tests;

fn rule(resource: &str, attribute: &str) -> Option<(&'static str, bool, &'static str)> {
    if resource != "aws_opensearchserverless_vpc_endpoint" {
        return None;
    }
    match attribute {
        "vpc_id" => Some(("aws_vpc", false, "Endpoint VPC")),
        "subnet_ids" => Some(("aws_subnet", true, "Configured subnets")),
        "security_group_ids" => Some(("aws_security_group", true, "Configured security groups")),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.2)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let Some((kind, collection, _)) = rule(&nodes[r.target].resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !super::service_evidence::eligible(nodes, r)
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains("id")
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let Some(actual) = super::streaming::values(value, unknown, &r.attribute, collection)
        else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == kind)
                    .then(|| super::service_evidence::known(evidence, source, "id"))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_opensearchserverless_vpc_endpoint" {
            continue;
        }
        let proof = |attribute: &str| {
            let mut refs = raw
                .attributes
                .iter()
                .filter(|r| r.target == target && r.attribute == attribute);
            let r = refs.next()?;
            (refs.next().is_none() && super::service_references::verified(raw, r)).then_some(r)
        };
        let Some((vpc, (subnets, groups))) =
            proof("vpc_id").zip(proof("subnet_ids").zip(proof("security_group_ids")))
        else {
            continue;
        };
        let [parent] = vpc.sources.as_slice() else {
            continue;
        };
        let members = subnets.sources.iter().chain(&groups.sources);
        if !members
            .clone()
            .all(|&member| super::streaming::vpc(raw, member) == Some(*parent))
        {
            continue;
        }
        let mut evidence = vec![relationships::reference(graph, *parent, target)];
        for &member in members {
            evidence.push(relationships::reference(graph, *parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, *parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*parent, target))
        });
    }
}
