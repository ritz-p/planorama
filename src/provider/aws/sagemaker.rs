use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/sagemaker.rs"]
mod tests;

const MODELS: &str = "production_variants.model_name";
const SUBNETS: &str = "vpc_config.subnets";
const GROUPS: &str = "vpc_config.security_group_ids";

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        ("aws_sagemaker_endpoint", "endpoint_config_name") => Some((
            "aws_sagemaker_endpoint_configuration",
            "name",
            false,
            "Endpoint configuration",
        )),
        ("aws_sagemaker_endpoint_configuration", MODELS) => {
            Some(("aws_sagemaker_model", "name", true, "Production models"))
        }
        ("aws_sagemaker_model", "execution_role_arn") => {
            Some(("aws_iam_role", "arn", false, "Execution role reference"))
        }
        ("aws_sagemaker_model", SUBNETS) => Some(("aws_subnet", "id", true, "Configured subnets")),
        ("aws_sagemaker_model", GROUPS) => Some((
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

fn models<'a>(value: &'a Value, unknown: &'a Value) -> Option<BTreeSet<&'a str>> {
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let entries = value["production_variants"].as_array()?;
    if entries.is_empty() {
        return None;
    }
    let masks = match &unknown["production_variants"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(masks) if masks.len() == entries.len() => Some(masks),
        _ => return None,
    };
    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let mask = masks.map_or(&Value::Null, |m| &m[i]);
            super::streaming::values(entry, mask, "model_name", false)?
                .first()
                .copied()
        })
        .collect()
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    let consumer = &raw.nodes[reference.target];
    reference.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            consumer.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            consumer
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role",
        )
    })
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
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(field)
            || !r.sources.iter().all(|&i| {
                let endpoint = &nodes[i];
                current(endpoint)
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
                    )
            })
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let actual = if r.attribute == MODELS {
            models(value, unknown)
        } else {
            super::streaming::values(value, unknown, &r.attribute, collection)
        };
        let Some(actual) = actual else { continue };
        let expected = r
            .sources
            .iter()
            .map(|&i| super::service_evidence::known(evidence, i, field))
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_sagemaker_model" {
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
        let Some((subnets, groups)) = proof(SUBNETS).zip(proof(GROUPS)) else {
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
        let evidence = members
            .flat_map(|&i| {
                [
                    relationships::reference(graph, parent, i),
                    relationships::reference(graph, i, target),
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
