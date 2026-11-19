use super::{GUARDRAIL, VERSION, compatible, known};
use crate::model::{AttributeReference, TerraformEntity};
use crate::provider::aws::{lambda_core, nested_references, route53};
use serde_json::Value;
pub(super) fn published(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        !v.is_empty()
            && v.bytes().all(|b| b.is_ascii_digit())
            && v.parse::<u64>().is_ok_and(|n| n > 0)
    })
}
pub(super) fn enforced(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let Some(version) = known(e, r.target, "guardrail_version").filter(|v| published(Some(v)))
    else {
        return false;
    };
    let Some(arn) = known(e, r.target, "guardrail_arn") else {
        return false;
    };
    let Some(vref) = lambda_core::unique(refs, r.target, "guardrail_version") else {
        return false;
    };
    let Some(s) = route53::endpoint(nodes, vref, VERSION, &["version"]) else {
        return false;
    };
    if known(e, s, "version") != Some(version)
        || known(e, s, "guardrail_arn") != Some(arn)
        || !compatible(&nodes[r.target], &nodes[s])
    {
        return false;
    }
    let Some(owner) = lambda_core::unique(refs, s, "guardrail_arn") else {
        return false;
    };
    let Some(parent) = route53::endpoint(nodes, owner, GUARDRAIL, &["guardrail_arn"]) else {
        return false;
    };
    if !compatible(&nodes[s], &nodes[parent])
        || nested_references::verify(nodes, refs, e, owner, super::fields, |_, _| true).is_none()
    {
        return false;
    }
    if r.attribute == "guardrail_version" {
        r.sources == [s]
    } else {
        r.sources == [parent] || r.sources == [s]
    }
}
