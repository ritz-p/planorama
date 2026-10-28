use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/fsx.rs"]
mod tests;

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        ("aws_fsx_lustre_file_system", "subnet_ids") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_fsx_lustre_file_system", "security_group_ids") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_fsx_lustre_file_system", "kms_key_id") => {
            Some(("aws_kms_key", "arn", false, "Encryption key reference"))
        }
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
        if r.attribute == "kms_key_id"
            && !matches!(
                super::service_evidence::known(evidence, r.target, "deployment_type"),
                Some("PERSISTENT_1" | "PERSISTENT_2")
            )
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let Some(actual) = super::streaming::values(value, unknown, &r.attribute, collection)
        else {
            continue;
        };
        if r.attribute == "subnet_ids" && actual.len() != 1 {
            continue;
        }
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == kind)
                    .then(|| super::service_evidence::known(evidence, source, field))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_fsx_lustre_file_system" {
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
        let Some((subnets, groups)) = proof("subnet_ids").zip(proof("security_group_ids")) else {
            continue;
        };
        let members = subnets.sources.iter().chain(&groups.sources);
        let Some(parents) = members
            .clone()
            .map(|&i| super::streaming::vpc(raw, i))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().expect("one VPC");
        let mut evidence = Vec::new();
        for &member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
    }
}
