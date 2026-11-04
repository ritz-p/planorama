use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront_functions.rs"]
mod tests;

const FUNCTION: &str = "aws_cloudfront_function";
const CONNECTION: &str = "aws_cloudfront_connection_function";
const STORE: &str = "aws_cloudfront_key_value_store";
const NESTED: &str = "connection_function_config.key_value_store_association.key_value_store_arn";

pub(super) fn helper(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudfrontkeyvaluestore_key" | "aws_cloudfrontkeyvaluestore_keys_exclusive"
    )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (FUNCTION, "key_value_store_associations") | (CONNECTION, NESTED) => {
            Some("Function KeyValueStore")
        }
        (kind, "key_value_store_arn") if helper(kind) => Some("Owner KeyValueStore"),
        _ => None,
    }
}

pub(super) fn live(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    source: usize,
) -> bool {
    if !matches!(nodes[source].resource_type.as_str(), FUNCTION | CONNECTION) {
        return true;
    }
    let (value, mask) = evidence[source];
    matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&mask["publish"], Value::Null | Value::Bool(false))
        && value["publish"].as_bool() == Some(true)
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = BTreeMap::new();
    for r in refs.iter() {
        *counts.entry((r.target, r.attribute.clone())).or_insert(0) += 1;
    }
    for r in refs {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
        {
            continue;
        }
        let (v, m) = evidence[r.target];
        let collection = node.resource_type == FUNCTION;
        if collection
            && super::service_evidence::known(evidence, r.target, "runtime")
                != Some("cloudfront-js-2.0")
        {
            continue;
        }
        if node.resource_type == CONNECTION {
            if v["connection_function_config"]
                .as_array()
                .is_none_or(|a| a.len() != 1)
                || v["connection_function_config"][0]["key_value_store_association"]
                    .as_array()
                    .is_none_or(|a| a.len() != 1)
                || super::appsync::scalar(evidence, r.target, "connection_function_config.runtime")
                    != Some("cloudfront-js-2.0")
            {
                continue;
            }
        }
        let Some(actual) = super::streaming::values(v, m, &r.attribute, collection) else {
            continue;
        };
        if !collection && r.sources.len() != 1 {
            continue;
        }
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                let source = &nodes[s];
                let attrs = r.source_attributes_by_instance.get(&s)?;
                if source.resource_type != STORE
                    || !current(source)
                    || !super::route53::configured(node, source)
                    || attrs.len() != 1
                    || !attrs.contains("arn")
                {
                    return None;
                }
                super::service_evidence::known(evidence, s, "arn")
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let incoming =
        super::cloudfront::reference_label(&raw.nodes[r.target].resource_type, &r.attribute)
            .is_some()
            && r.sources
                .iter()
                .any(|&s| matches!(raw.nodes[s].resource_type.as_str(), FUNCTION | CONNECTION));
    (label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() && !incoming)
        || super::route53::bindings(raw, r)
}
