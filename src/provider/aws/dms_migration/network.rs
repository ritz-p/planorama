use crate::model::{AttributeReference, TerraformEntity};
use crate::provider::aws::{lambda_core, nested_references};
use serde_json::Value;
use std::collections::BTreeSet;
fn context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    let group = lambda_core::unique(refs, t, "subnet_group_identifier")?;
    let groups = lambda_core::unique(refs, t, "vpc_security_group_ids")?;
    if !group.service_reference_verified || !groups.service_reference_verified {
        return None;
    }
    let [s] = group.sources.as_slice() else {
        return None;
    };
    let subnets = lambda_core::unique(refs, *s, "subnet_ids")?;
    nested_references::verify(nodes, refs, e, subnets, super::fields, |member, _| {
        super::compatible(&nodes[t], &nodes[member])
    })
    .filter(|_| subnets.service_reference_verified)?;
    let parents = subnets
        .sources
        .iter()
        .chain(&groups.sources)
        .map(|&member| {
            let parent = lambda_core::parent(nodes, refs, e, member)?;
            (super::compatible(&nodes[t], &nodes[parent])
                && super::compatible(&nodes[member], &nodes[parent]))
            .then_some(parent)
        })
        .collect::<Option<BTreeSet<_>>>()?;
    (parents.len() == 1).then(|| *parents.first().unwrap())
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            (nodes[r.target].resource_type == super::PROFILE
                && matches!(
                    r.attribute.as_str(),
                    "subnet_group_identifier" | "vpc_security_group_ids"
                ))
            .then(|| {
                context(nodes, refs, e, r.target)
                    .into_iter()
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for (r, context) in refs.iter_mut().zip(contexts) {
        if let Some(context) = context {
            r.service_reference_verified &= context.len() == 1;
            r.endpoint_network_context = context;
        }
    }
}
