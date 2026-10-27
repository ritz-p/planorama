use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/apprunner.rs"]
mod tests;

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        (
            "aws_apprunner_service",
            "network_configuration.egress_configuration.vpc_connector_arn",
        ) => Some((
            "aws_apprunner_vpc_connector",
            "arn",
            false,
            "Egress connector",
        )),
        ("aws_apprunner_vpc_ingress_connection", "service_arn") => {
            Some(("aws_apprunner_service", "arn", false, "Ingress service"))
        }
        ("aws_apprunner_vpc_connector", "subnets") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_apprunner_vpc_connector", "security_groups") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.3)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !super::service_evidence::eligible(nodes, r)
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(field)
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        if node.resource_type == "aws_apprunner_service"
            && super::streaming::values(
                value,
                unknown,
                "network_configuration.egress_configuration.egress_type",
                false,
            ) != Some(["VPC"].into())
        {
            continue;
        }
        let Some(actual) = super::streaming::values(value, unknown, &r.attribute, collection)
        else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == kind)
                    .then(|| super::service_evidence::known(evidence, source, field))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    reference.sources.iter().all(|&source| {
        let node = &raw.nodes[reference.target];
        let endpoint = &raw.nodes[source];
        super::service_references::cluster_binding_matches(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
        )
    })
}

fn network(raw: &Input, target: usize) -> Option<(usize, Vec<usize>)> {
    let proof = |attribute: &str| {
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == attribute);
        let r = refs.next()?;
        (refs.next().is_none() && r.service_reference_verified && bindings(raw, r)).then_some(r)
    };
    let (subnets, groups) = proof("subnets").zip(proof("security_groups"))?;
    let members = subnets
        .sources
        .iter()
        .chain(&groups.sources)
        .copied()
        .collect::<Vec<_>>();
    let parents = members
        .iter()
        .map(|&i| super::streaming::vpc(raw, i))
        .collect::<Option<BTreeSet<_>>>()?;
    (parents.len() == 1).then(|| (*parents.first().expect("one VPC"), members))
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_apprunner_vpc_connector" {
            continue;
        }
        let Some((parent, members)) = network(raw, target) else {
            continue;
        };
        let evidence = members
            .into_iter()
            .flat_map(|member| {
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
