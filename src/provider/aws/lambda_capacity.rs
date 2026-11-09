use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_capacity.rs"]
mod tests;

const KIND: &str = "aws_lambda_capacity_provider";
const SUBNETS: &str = "vpc_config.subnet_ids";
const GROUPS: &str = "vpc_config.security_group_ids";
const ROLE: &str = "permissions_config.capacity_provider_operator_role_arn";

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool, &'static str)> {
    if kind != KIND {
        return None;
    }
    match path {
        SUBNETS => Some(("aws_subnet", "id", true, "Capacity subnets")),
        GROUPS => Some(("aws_security_group", "id", true, "Capacity security groups")),
        ROLE => Some(("aws_iam_role", "arn", false, "Capacity operator role")),
        "kms_key_arn" => Some(("aws_kms_key", "arn", false, "Capacity encryption key")),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.3)
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
                if super::lambda_core::unique(refs, r.target, &r.attribute).is_none()
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
                let block = if collection {
                    Some("vpc_config")
                } else if r.attribute == ROLE {
                    Some("permissions_config")
                } else {
                    None
                };
                if block.is_some_and(|block| value[block].as_array().is_none_or(|a| a.len() != 1)) {
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
                (actual == expected && actual.len() == r.sources.len()).then_some(())
            };
            let verified = proof().is_some();
            let context = if verified && collection {
                r.sources
                    .iter()
                    .map(|&s| super::lambda_core::parent(nodes, refs, evidence, s))
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
    r.sources.iter().all(|&source| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[source];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            r.attribute == ROLE,
        )
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(raw, graph, KIND, (SUBNETS, GROUPS), bindings);
}
