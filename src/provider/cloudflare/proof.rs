use super::evidence::{Evidence, Result, collect};
use crate::model::{Action, AttributeReference, CloudflareFailure as Failure, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn active(node: &TerraformEntity) -> bool {
    node.provider.is_cloudflare()
        && node.deposed_key.is_none()
        && node.action != Action::Delete
        && node.metadata.state_removal.is_none()
}

pub(super) fn current(node: &TerraformEntity) -> bool {
    active(node) && super::known(&node.resource_type, node.mode)
}

pub(super) fn known_configuration_matches(a: &TerraformEntity, b: &TerraformEntity) -> bool {
    a.provider_configuration
        .as_ref()
        .is_some_and(|configuration| Some(configuration) == b.provider_configuration.as_ref())
}

pub(super) fn unique<T: Ord>(values: &[T]) -> Result<()> {
    (values.iter().collect::<BTreeSet<_>>().len() == values.len())
        .then_some(())
        .ok_or(Failure::IncompleteCollection)
}

pub(super) fn endpoint(node: &TerraformEntity) -> Result<()> {
    if !node.provider.is_cloudflare() || !super::known(&node.resource_type, node.mode) {
        Err(Failure::UnsupportedContract)
    } else if !current(node) {
        Err(Failure::InactiveEndpoint)
    } else {
        Ok(())
    }
}

pub(super) fn unmasked(mask: &Value) -> bool {
    match mask {
        Value::Null | Value::Bool(false) => true,
        Value::Object(fields) => fields.values().all(unmasked),
        Value::Array(values) => values.iter().all(unmasked),
        _ => false,
    }
}

pub(super) fn scope_failure(node: &TerraformEntity, evidence: Evidence<'_>) -> Failure {
    let Some((_, _, account, zone)) = super::types::TYPES
        .iter()
        .find(|&&(name, mode, _, _)| name == node.resource_type && mode == node.mode)
    else {
        return Failure::UnsupportedContract;
    };
    let failures = [account, zone]
        .into_iter()
        .filter(|path| !path.is_empty())
        .filter_map(|path| evidence.id(&path.join(".")).err())
        .filter(|failure| matches!(failure, Failure::UnknownValue | Failure::SensitiveValue));
    failures.min().unwrap_or(Failure::ScopeUnavailable)
}

pub(super) fn account(
    a: &TerraformEntity,
    b: &TerraformEntity,
    av: Evidence<'_>,
    bv: Evidence<'_>,
) -> Result<()> {
    let first = a
        .cloudflare_scope
        .as_ref()
        .and_then(|s| s.account.as_ref())
        .ok_or_else(|| scope_failure(a, av));
    let second = b
        .cloudflare_scope
        .as_ref()
        .and_then(|s| s.account.as_ref())
        .ok_or_else(|| scope_failure(b, bv));
    match (first, second) {
        (Ok(a), Ok(b)) => (a == b).then_some(()).ok_or(Failure::ScopeMismatch),
        (Err(a), Err(b)) => Err(a.min(b)),
        (Err(reason), _) | (_, Err(reason)) => Err(reason),
    }
}

pub(super) struct Target<'a> {
    pub value: &'a str,
    pub kind: &'static str,
    pub output: &'static str,
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
    targets: &[Target<'_>],
    compatible: impl Fn(usize, usize) -> Result<()>,
) -> Result<()> {
    explicit(nodes, refs, r)?;
    if targets.is_empty() {
        return Err(Failure::UnsupportedContract);
    }
    let sources: BTreeSet<_> = collect(targets.iter().map(|target| {
        let matches = |index: usize| -> Result<()> {
            let node = &nodes[index];
            endpoint(node)?;
            if node.resource_type != target.kind {
                return Err(Failure::UnsupportedContract);
            }
            compatible(r.target, index)?;
            (evidence[index].text(target.output)? == target.value)
                .then_some(())
                .ok_or(Failure::IdentityMismatch)
        };
        let mut candidates = (0..nodes.len()).filter(|&index| matches(index).is_ok());
        let Some(source) = candidates.next() else {
            return Err(r
                .sources
                .iter()
                .filter(|&&index| nodes[index].resource_type == target.kind)
                .filter_map(|&index| {
                    if r.source_attributes_by_instance
                        .get(&index)
                        .is_some_and(|attrs| attrs.len() != 1 || !attrs.contains(target.output))
                    {
                        Some(Failure::OutputMismatch)
                    } else {
                        matches(index).err()
                    }
                })
                .min()
                .unwrap_or(Failure::UnsupportedContract));
        };
        if candidates.next().is_some() {
            return Err(Failure::AmbiguousEndpoint);
        }
        if source == r.target || !r.sources.contains(&source) {
            return Err(Failure::IncompleteProvenance);
        }
        let attributes = r
            .source_attributes_by_instance
            .get(&source)
            .ok_or(Failure::IncompleteProvenance)?;
        if attributes.len() != 1 || !attributes.contains(target.output) {
            return Err(Failure::OutputMismatch);
        }
        Ok(source)
    }))?
    .into_iter()
    .collect();
    (sources == r.sources.iter().copied().collect())
        .then_some(())
        .ok_or(Failure::IncompleteProvenance)
}

pub(super) fn explicit(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
) -> Result<()> {
    endpoint(&nodes[r.target])?;
    (r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && refs
            .iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            == 1)
        .then_some(())
        .ok_or(Failure::IncompleteProvenance)
}

pub(super) fn record(r: &mut AttributeReference, result: Result<()>) {
    r.service_reference_verified = result.is_ok();
    r.cloudflare_failure = result.err();
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_proof.rs"]
mod tests;
