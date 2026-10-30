use crate::model::{
    Action, AttributeReference, EntityMode, Graph, ReferenceKind, ReferenceTarget,
    ResourceReference, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront_vpc_origin.rs"]
mod tests;

const PATH: &str = "vpc_origin_endpoint_config.arn";

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn eligible(nodes: &[TerraformEntity], r: &AttributeReference) -> bool {
    current(&nodes[r.target])
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && !r.sources.is_empty()
        && r.sources.iter().all(|&s| {
            s != r.target
                && current(&nodes[s])
                && super::cloudfront::compatible(&nodes[r.target], &nodes[s])
        })
}

fn verify(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<usize> {
    let [source] = r.sources.as_slice() else {
        return None;
    };
    if !eligible(nodes, r) || r.source_attributes != ["arn".to_owned()].into() {
        return None;
    }
    let endpoint = &nodes[*source];
    let kind = super::aliases::entity_type(endpoint);
    if !matches!(kind, "aws_lb" | "aws_instance") || super::scope::scope(endpoint)?.region.is_none()
    {
        return None;
    }
    if kind == "aws_lb"
        && !matches!(
            super::service_evidence::known(evidence, *source, "load_balancer_type"),
            Some("application" | "network")
        )
    {
        return None;
    }
    let (value, mask) = evidence[r.target];
    if value["vpc_origin_endpoint_config"]
        .as_array()
        .is_none_or(|v| v.len() != 1)
    {
        return None;
    }
    let arn =
        super::service_references::scalar(value, mask, &["vpc_origin_endpoint_config", "arn"])?;
    let block_mask = &mask["vpc_origin_endpoint_config"];
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(
            block_mask,
            Value::Null | Value::Bool(false) | Value::Array(_)
        )
        || block_mask.as_array().is_some_and(|m| m.len() != 1)
        || !matches!(
            &block_mask[0],
            Value::Null | Value::Bool(false) | Value::Object(_)
        )
        || !matches!(&block_mask[0]["arn"], Value::Null | Value::Bool(false))
    {
        return None;
    }
    (Some(arn) == super::service_evidence::known(evidence, *source, "arn")).then_some(*source)
}

fn context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    endpoint: usize,
) -> Option<Vec<usize>> {
    let region = super::scope::scope(&nodes[endpoint])?.region?;
    let kind = super::aliases::entity_type(&nodes[endpoint]);
    let field = if kind == "aws_instance" {
        "subnet_id"
    } else {
        "subnets"
    };
    let mut found = refs
        .iter()
        .filter(|r| r.target == endpoint && r.attribute == field);
    let reference = match found.next() {
        Some(reference) => reference,
        None if kind == "aws_lb" => {
            let mut mappings = refs
                .iter()
                .filter(|r| r.target == endpoint && r.attribute == "subnet_mapping.subnet_id");
            let mapping = mappings.next()?;
            return (mappings.next().is_none()
                && mapping.service_reference_verified
                && eligible(nodes, mapping)
                && mapping.sources.iter().all(|&source| {
                    super::scope::scope(&nodes[source])
                        .and_then(|s| s.region)
                        .as_ref()
                        == Some(&region)
                }))
            .then(|| mapping.sources.clone());
        }
        None => return None,
    };
    if found.next().is_some() || !eligible(nodes, reference) || !reference.ids_only {
        return None;
    }
    let (value, mask) = evidence[endpoint];
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&mask[field], Value::Null | Value::Bool(false))
    {
        return None;
    }
    let actual: BTreeSet<_> = if kind == "aws_instance" {
        [value[field].as_str()?].into()
    } else {
        value[field]
            .as_array()?
            .iter()
            .map(Value::as_str)
            .collect::<Option<_>>()?
    };
    let mut expected = BTreeSet::new();
    let mut vpc = None;
    for &subnet in &reference.sources {
        let node = &nodes[subnet];
        if node.resource_type != "aws_subnet"
            || super::scope::scope(node)?.region.as_ref() != Some(&region)
        {
            return None;
        }
        expected.insert(super::service_evidence::known(evidence, subnet, "id")?);
        let subnet_vpc = super::service_evidence::known(evidence, subnet, "vpc_id")?;
        if vpc.is_some_and(|v| v != subnet_vpc) {
            return None;
        }
        vpc = Some(subnet_vpc);
    }
    if super::service_evidence::known(evidence, endpoint, "vpc_id")
        .is_some_and(|id| Some(id) != vpc)
    {
        return None;
    }
    if actual.is_empty() || actual != expected {
        return None;
    }
    Some(reference.sources.clone())
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != "aws_cloudfront_vpc_origin" || r.attribute != PATH {
                return None;
            }
            let endpoint = verify(nodes, evidence, r);
            Some((
                endpoint.is_some(),
                endpoint
                    .and_then(|s| context(nodes, refs, evidence, s))
                    .unwrap_or_default(),
            ))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((verified, context)) = result {
            r.service_reference_verified = verified;
            r.endpoint_network_context = context;
        }
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for r in &raw.attributes {
        if raw.nodes[r.target].resource_type != "aws_cloudfront_vpc_origin"
            || r.attribute != PATH
            || !r.service_reference_verified
            || r.endpoint_network_context.is_empty()
        {
            continue;
        }
        graph.nodes[r.target].references.push(ResourceReference {
            kind: ReferenceKind::Information,
            preserve_spatial: false,
            show_spatial: true,
            label: "Endpoint subnet context",
            attribute: PATH.to_owned(),
            targets: r
                .endpoint_network_context
                .iter()
                .map(|&s| ReferenceTarget {
                    account_scope: None,
                    state: None,
                    id: raw.nodes[s].entity.provenance[0].clone(),
                    placement: raw.nodes[s].placement,
                })
                .collect(),
            complete: true,
            dependency: false,
            issues: Default::default(),
        });
    }
}
