use super::appsync::scalar;
use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const FUNCTION: &str = "aws_appsync_function";
const RESOLVER: &str = "aws_appsync_resolver";
const PIPELINE: &str = "pipeline_config.functions";
const CONFLICT: &str = "sync_config.lambda_conflict_handler_config.lambda_conflict_handler_arn";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if !matches!(kind, FUNCTION | RESOLVER) {
        return None;
    }
    match path {
        "api_id" => Some("Execution GraphQL API"),
        "data_source" => Some("API-qualified data source"),
        CONFLICT => Some("Lambda conflict handler"),
        PIPELINE if kind == RESOLVER => Some("Ordered pipeline functions"),
        _ => None,
    }
}

fn compatible(nodes: &[TerraformEntity], a: usize, b: usize) -> bool {
    let region = super::scope::scope(&nodes[a]).and_then(|s| s.region);
    region.is_some()
        && region == super::scope::scope(&nodes[b]).and_then(|s| s.region)
        && super::route53::configured(&nodes[a], &nodes[b])
}

fn parent(refs: &[AttributeReference], index: usize) -> Option<&AttributeReference> {
    let mut parents = refs.iter().filter(|r| {
        r.target == index
            && r.attribute == "api_id"
            && r.service_reference_verified
            && r.sources.len() == 1
    });
    let parent = parents.next()?;
    parents.next().is_none().then_some(parent)
}

fn same_api(refs: &[AttributeReference], a: usize, b: usize) -> bool {
    parent(refs, a)
        .zip(parent(refs, b))
        .is_some_and(|(a, b)| a.sources == b.sources)
}

fn known<'a>(v: &'a Value, m: &Value, path: &[&str]) -> Option<&'a Value> {
    if path.is_empty() {
        let clear = matches!(m, Value::Null | Value::Bool(false))
            || v.as_array().zip(m.as_array()).is_some_and(|(v, m)| {
                v.len() == m.len()
                    && m.iter()
                        .all(|m| matches!(m, Value::Null | Value::Bool(false)))
            });
        return clear.then_some(v);
    }
    if let Some(rows) = v.as_array() {
        let [row] = rows.as_slice() else { return None };
        let mask = match m {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(a) if a.len() == 1 => &a[0],
            _ => return None,
        };
        return known(row, mask, path);
    }
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    known(&v[path[0]], &m[path[0]], &path[1..])
}

fn absent(v: &Value, m: &Value, path: &[&str]) -> bool {
    known(v, m, path).is_some_and(|v| {
        v.is_null() || v.as_str() == Some("") || v.as_array().is_some_and(Vec::is_empty)
    })
}

fn pipeline(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    if scalar(evidence, r.target, "kind") != Some("PIPELINE")
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains("function_id")
    {
        return None;
    }
    let (v, m) = evidence[r.target];
    if !absent(v, m, &["data_source"]) {
        return None;
    }
    let values = known(v, m, &["pipeline_config", "functions"])?
        .as_array()
        .filter(|v| !v.is_empty())?;
    let mut endpoints = BTreeMap::new();
    for &s in &r.sources {
        let n = &nodes[s];
        if n.resource_type != FUNCTION
            || !n.provider.is_aws()
            || n.mode != EntityMode::Managed
            || n.action == Action::Delete
            || n.deposed_key.is_some()
            || !compatible(nodes, r.target, s)
            || !same_api(refs, r.target, s)
        {
            return None;
        }
        if endpoints
            .insert(scalar(evidence, s, "function_id")?, s)
            .is_some()
        {
            return None;
        }
    }
    let ordered: Vec<_> = values
        .iter()
        .map(|v| endpoints.get(v.as_str()?).copied())
        .collect::<Option<_>>()?;
    (ordered.iter().copied().collect::<BTreeSet<_>>() == r.sources.iter().copied().collect())
        .then_some(ordered)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs
        .iter_mut()
        .filter(|r| label(&nodes[r.target].resource_type, &r.attribute).is_some())
    {
        r.service_reference_verified = false;
        if r.attribute != "api_id" {
            continue;
        }
        let Some(s) = super::route53::endpoint(nodes, r, super::appsync::API, &["id"]) else {
            continue;
        };
        r.service_reference_verified = compatible(nodes, r.target, s)
            && scalar(evidence, r.target, "api_id").is_some()
            && scalar(evidence, r.target, "api_id") == scalar(evidence, s, "id");
    }
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = nodes[r.target].resource_type.as_str();
            if label(kind, &r.attribute).is_none()
                || r.attribute == "api_id"
                || parent(refs, r.target).is_none()
            {
                return None;
            }
            if kind == RESOLVER
                && (scalar(evidence, r.target, "type").is_none()
                    || scalar(evidence, r.target, "field").is_none())
            {
                return None;
            }
            if r.attribute == PIPELINE {
                return pipeline(nodes, refs, evidence, r);
            }
            let (target, output) = match r.attribute.as_str() {
                "data_source" => {
                    let (v, m) = evidence[r.target];
                    if kind == RESOLVER
                        && (scalar(evidence, r.target, "kind") != Some("UNIT")
                            || !absent(v, m, &["pipeline_config"]))
                    {
                        return None;
                    }
                    ("aws_appsync_datasource", "name")
                }
                CONFLICT
                    if scalar(evidence, r.target, "sync_config.conflict_handler")
                        == Some("LAMBDA") =>
                {
                    ("aws_lambda_function", "arn")
                }
                _ => return None,
            };
            let s = super::route53::endpoint(nodes, r, target, &[output])?;
            if !compatible(nodes, r.target, s)
                || (r.attribute == "data_source" && !same_api(refs, r.target, s))
            {
                return None;
            }
            let value = scalar(evidence, r.target, &r.attribute)?;
            (scalar(evidence, s, output) == Some(value)).then_some(vec![s])
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(sources) = result {
            r.sources = sources;
            r.service_reference_verified = true;
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    if !super::route53::bindings(raw, r) {
        return false;
    }
    if r.attribute == "api_id" {
        return true;
    }
    let Some(api) = parent(&raw.attributes, r.target) else {
        return false;
    };
    if !super::route53::bindings(raw, api) {
        return false;
    }
    !matches!(r.attribute.as_str(), "data_source" | PIPELINE)
        || r.sources.iter().all(|&s| {
            parent(&raw.attributes, s)
                .is_some_and(|p| p.sources == api.sources && super::route53::bindings(raw, p))
        })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync_execution.rs"]
mod tests;
