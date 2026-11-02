use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/api_usage.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_api_gateway_usage_plan", "api_stages.api_id") => Some("Usage plan APIs"),
        ("aws_api_gateway_usage_plan", "api_stages.stage") => Some("API-qualified stages"),
        ("aws_api_gateway_usage_plan_key", "key_id") => Some("Usage API key"),
        ("aws_api_gateway_usage_plan_key", "usage_plan_id") => Some("Usage plan"),
        _ => None,
    }
}

fn reference<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    field: &str,
) -> Option<&'a AttributeReference> {
    let mut matches = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = matches.next()?;
    matches.next().is_none().then_some(r)
}

fn endpoints(nodes: &[TerraformEntity], r: &AttributeReference, kind: &str, output: &str) -> bool {
    super::vpc_routes::eligible(nodes, r)
        && r.source_attributes.len() == 1
        && r.source_attributes.contains(output)
        && r.sources.iter().all(|&s| nodes[s].resource_type == kind)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let known = |i, field| super::service_evidence::known(evidence, i, field);
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = nodes[r.target].resource_type.as_str();
            if label(kind, &r.attribute).is_none() {
                return r.service_reference_verified;
            }
            if kind == "aws_api_gateway_usage_plan_key" {
                let endpoint = if r.attribute == "key_id" {
                    "aws_api_gateway_api_key"
                } else {
                    "aws_api_gateway_usage_plan"
                };
                return known(r.target, "key_type") == Some("API_KEY")
                    && endpoints(nodes, r, endpoint, "id")
                    && r.sources.len() == 1
                    && known(r.target, &r.attribute).is_some()
                    && known(r.target, &r.attribute) == known(r.sources[0], "id");
            }
            let Some(apis) = reference(refs, r.target, "api_stages.api_id") else {
                return false;
            };
            let Some(stages) = reference(refs, r.target, "api_stages.stage") else {
                return false;
            };
            if !endpoints(nodes, apis, "aws_api_gateway_rest_api", "id")
                || !endpoints(nodes, stages, "aws_api_gateway_stage", "stage_name")
            {
                return false;
            }
            let (value, unknown) = evidence[r.target];
            if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
                return false;
            }
            let Some(rows) = value["api_stages"]
                .as_array()
                .filter(|rows| !rows.is_empty())
            else {
                return false;
            };
            let masks = match &unknown["api_stages"] {
                Value::Null | Value::Bool(false) => None,
                Value::Array(masks) if masks.len() == rows.len() => Some(masks),
                _ => return false,
            };
            let mut used_apis = BTreeSet::new();
            let mut used_stages = BTreeSet::new();
            for (index, row) in rows.iter().enumerate() {
                let mask = masks.map_or(&Value::Null, |masks| &masks[index]);
                let row_evidence = [(row, mask)];
                let Some(api_id) = super::service_evidence::known(&row_evidence, 0, "api_id")
                else {
                    return false;
                };
                let Some(name) = super::service_evidence::known(&row_evidence, 0, "stage") else {
                    return false;
                };
                let mut candidates = apis
                    .sources
                    .iter()
                    .filter(|&&s| known(s, "id") == Some(api_id));
                let Some(&api) = candidates.next() else {
                    return false;
                };
                if candidates.next().is_some() {
                    return false;
                }
                let mut candidates = stages.sources.iter().filter(|&&s| {
                    known(s, "stage_name") == Some(name)
                        && reference(refs, s, "rest_api_id").is_some_and(|parent| {
                            parent.service_reference_verified && parent.sources.as_slice() == [api]
                        })
                });
                let Some(&stage) = candidates.next() else {
                    return false;
                };
                if candidates.next().is_some() || !used_stages.insert(stage) {
                    return false;
                }
                used_apis.insert(api);
            }
            used_apis.len() == apis.sources.len() && used_stages.len() == stages.sources.len()
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = valid;
        }
    }
}
