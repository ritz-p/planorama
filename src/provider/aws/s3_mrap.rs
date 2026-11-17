use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

const MRAP: &str = "aws_s3control_multi_region_access_point";
const POLICY: &str = "aws_s3control_multi_region_access_point_policy";
const ROUTES: &str = "aws_s3control_multi_region_access_point_routes";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (MRAP, "details.region.bucket") => Some("MRAP buckets"),
        (POLICY, "details.name") => Some("MRAP policy reference"),
        (ROUTES, "mrap") => Some("MRAP routes reference"),
        (ROUTES, "route.bucket") => Some("MRAP route buckets"),
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let parts: Vec<_> = path.split('.').collect();
    match (kind, parts.as_slice()) {
        (MRAP, ["details", d, "region", i, "bucket"]) => *d == "0" && i.parse::<usize>().is_ok(),
        (ROUTES, ["route", i, "bucket"]) => i.parse::<usize>().is_ok(),
        _ => false,
    }
}

fn scalar<'a>(v: &'a Value, m: &Value, field: &str) -> Option<&'a str> {
    fn at<'a>(v: &'a Value, m: &Value, path: &[&str]) -> Option<&'a str> {
        if let Some(values) = v.as_array() {
            let [v] = values.as_slice() else { return None };
            let m = match m {
                Value::Null | Value::Bool(false) => &Value::Null,
                Value::Array(a) if a.len() == 1 => &a[0],
                _ => return None,
            };
            return at(v, m, path);
        }
        if let Some((field, rest)) = path.split_first() {
            if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
                return None;
            }
            return at(v.get(*field)?, &m[*field], rest);
        }
        matches!(m, Value::Null | Value::Bool(false))
            .then(|| v.as_str().filter(|s| !s.is_empty()))
            .flatten()
    }
    at(v, m, &field.split('.').collect::<Vec<_>>())
}

fn rows<'a>(v: &'a Value, m: &'a Value, kind: &str) -> Option<Vec<(&'a Value, &'a Value)>> {
    let (v, m) = if kind == MRAP {
        let details = block(v, m, "details")?;
        let [(v, m)] = details.as_slice() else {
            return None;
        };
        (*v, *m)
    } else {
        (v, m)
    };
    block(v, m, if kind == MRAP { "region" } else { "route" })
}

fn block<'a>(v: &'a Value, m: &'a Value, field: &str) -> Option<Vec<(&'a Value, &'a Value)>> {
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let values = v[field].as_array().filter(|a| !a.is_empty())?;
    let masks = match &m[field] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == values.len() => Some(a),
        _ => return None,
    };
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let m = masks.map_or(&Value::Null, |a| &a[i]);
            (v.is_object() && matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)))
                .then_some((v, m))
        })
        .collect()
}

fn account(v: &Value, m: &Value, field: &str, endpoint: &TerraformEntity) -> bool {
    super::vpc_gateways::absent(v, m, field)
        || scalar(v, m, field).is_some_and(|id| {
            crate::model::AwsAccountScope::new(id).is_some_and(|id| {
                endpoint
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
                    == Some(&id)
            })
        })
}

fn collection(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let kind = nodes[r.target].resource_type.as_str();
    let (v, m) = evidence[r.target];
    let Some(rows) = rows(v, m, kind) else {
        return false;
    };
    if rows.len() > 20 && kind == MRAP {
        return false;
    }
    let mut sources = BTreeSet::new();
    let mut names = BTreeSet::new();
    for i in 0..rows.len() {
        let path = if kind == MRAP {
            format!("details.0.region.{i}.bucket")
        } else {
            format!("route.{i}.bucket")
        };
        let mut found = refs
            .iter()
            .filter(|a| a.target == r.target && a.attribute == path);
        let Some(a) = found.next() else { return false };
        let Some(s) = super::route53::endpoint(nodes, a, "aws_s3_bucket", &["id", "bucket"]) else {
            return false;
        };
        let lengths = if kind == MRAP {
            vec![1, rows.len()]
        } else {
            vec![rows.len()]
        };
        if found.next().is_some()
            || a.indexed_block_lengths != lengths
            || !sources.insert(s)
            || !super::route53::configured(&nodes[r.target], &nodes[s])
        {
            return false;
        }
        let (sv, sm) = evidence[s];
        let Some(name) = scalar(sv, sm, a.source_attributes.first().unwrap()) else {
            return false;
        };
        if !names.insert(name) {
            return false;
        }
        let mut matched = rows
            .iter()
            .filter(|(v, m)| scalar(v, m, "bucket") == Some(name));
        let Some((row, mask)) = matched.next() else {
            return false;
        };
        if matched.next().is_some() || !account(row, mask, "bucket_account_id", &nodes[s]) {
            return false;
        }
        if kind == ROUTES {
            let region = super::scope::scope(&nodes[s]).and_then(|s| s.region);
            if region.as_ref().map(|r| r.0.as_str()) != scalar(row, mask, "region")
                || region.is_none()
            {
                return false;
            }
            let Some(dial) = super::service_evidence::known_value(
                &[(*row, *mask)],
                0,
                &["traffic_dial_percentage"],
            )
            .and_then(Value::as_u64) else {
                return false;
            };
            if dial > 100 {
                return false;
            }
        }
    }
    r.issues.is_empty() && r.instances_explicit && sources == r.sources.iter().copied().collect()
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let kind = nodes[r.target].resource_type.as_str();
    if matches!(
        r.attribute.as_str(),
        "details.region.bucket" | "route.bucket"
    ) {
        return collection(nodes, refs, evidence, r);
    }
    let field = if kind == POLICY { "name" } else { "arn" };
    let Some(s) = super::route53::endpoint(nodes, r, MRAP, &[field]) else {
        return false;
    };
    let (v, m) = evidence[r.target];
    let (sv, sm) = evidence[s];
    if !super::route53::configured(&nodes[r.target], &nodes[s])
        || scalar(v, m, &r.attribute).is_none()
        || scalar(v, m, &r.attribute) != scalar(sv, sm, field)
        || !account(v, m, "account_id", &nodes[s])
    {
        return false;
    }
    if kind == POLICY {
        let Some(id) = scalar(v, m, "account_id") else {
            return false;
        };
        if scalar(sv, sm, "account_id") != Some(id) {
            return false;
        }
    }
    true
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).is_some()
                && verify(nodes, refs, evidence, r)
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
            if verified {
                r.complete = true;
            }
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || super::route53::bindings(raw, r)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_mrap.rs"]
mod tests;
