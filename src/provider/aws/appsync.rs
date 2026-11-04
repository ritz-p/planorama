use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

pub(super) const API: &str = "aws_appsync_graphql_api";
const ASSOCIATION: &str = "aws_appsync_source_api_association";
const ADDITIONAL: &str = "additional_authentication_provider.user_pool_config.user_pool_id";

pub(super) fn scalar<'a>(
    evidence: &[(&'a Value, &Value)],
    index: usize,
    path: &str,
) -> Option<&'a str> {
    let (v, m) = evidence[index];
    super::service_references::scalar(v, m, &path.split('.').collect::<Vec<_>>())
}

pub(super) fn optional<'a>(v: &'a Value, m: &Value, path: &[&str]) -> Option<Option<&'a str>> {
    if let Some(rows) = v.as_array() {
        let [row] = rows.as_slice() else { return None };
        let mask = match m {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(a) if a.len() == 1 => &a[0],
            _ => return None,
        };
        return optional(row, mask, path);
    }
    if let Some((head, tail)) = path.split_first() {
        if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        return optional(&v[*head], &m[*head], tail);
    }
    if !matches!(m, Value::Null | Value::Bool(false)) {
        return None;
    }
    if v.is_null() || v.as_str() == Some("") {
        Some(None)
    } else {
        v.as_str().map(Some)
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if let Some(label) = super::appsync_domains::label(kind, path) {
        return Some(label);
    }
    if let Some(label) = super::appsync_events::label(kind, path) {
        return Some(label);
    }
    if let Some(label) = super::appsync_helpers::label(kind, path) {
        return Some(label);
    }
    if let Some(label) = super::appsync_execution::label(kind, path) {
        return Some(label);
    }
    if let Some(label) = super::appsync_datasource::label(kind, path) {
        return Some(label);
    }
    match (kind, path) {
        (API, "lambda_authorizer_config.authorizer_uri") => Some("GraphQL Lambda authorizer"),
        (API, "user_pool_config.user_pool_id" | ADDITIONAL) => Some("GraphQL Cognito user pool"),
        (API, "merged_api_execution_role_arn") => Some("Merged API execution role"),
        (API, "log_config.cloudwatch_logs_role_arn") => Some("GraphQL logging role"),
        (ASSOCIATION, "merged_api_id" | "merged_api_arn") => Some("Merged GraphQL API"),
        (ASSOCIATION, "source_api_id" | "source_api_arn") => Some("Source GraphQL API"),
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == API
        && path != ADDITIONAL
        && path
            .split('.')
            .filter(|p| p.parse::<usize>().is_err())
            .collect::<Vec<_>>()
            .join(".")
            == ADDITIONAL
}

fn additional(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let (v, m) = evidence[r.target];
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let Some(rows) = v["additional_authentication_provider"]
        .as_array()
        .filter(|a| !a.is_empty())
    else {
        return false;
    };
    let masks = match &m["additional_authentication_provider"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == rows.len() => Some(a),
        _ => return false,
    };
    let mut expected = std::collections::BTreeSet::new();
    let mut actual = std::collections::BTreeSet::new();
    for (i, row) in rows.iter().enumerate() {
        let mask = masks.map_or(&Value::Null, |a| &a[i]);
        let Some(auth) = super::service_references::scalar(row, mask, &["authentication_type"])
        else {
            return false;
        };
        if auth != "AMAZON_COGNITO_USER_POOLS" {
            continue;
        }
        let Some(id) =
            super::service_references::scalar(row, mask, &["user_pool_config", "user_pool_id"])
        else {
            return false;
        };
        if !actual.insert(id) {
            return false;
        }
    }
    for i in 0..rows.len() {
        let path =
            format!("additional_authentication_provider.{i}.user_pool_config.0.user_pool_id");
        let mut matches = refs
            .iter()
            .filter(|a| a.target == r.target && a.attribute == path);
        let Some(a) = matches.next() else { continue };
        let Some(s) = super::route53::endpoint(nodes, a, "aws_cognito_user_pool", &["id"]) else {
            return false;
        };
        if matches.next().is_some()
            || a.indexed_block_lengths != [rows.len(), 1]
            || !super::route53::configured(&nodes[r.target], &nodes[s])
        {
            return false;
        }
        let Some(id) = scalar(evidence, s, "id") else {
            return false;
        };
        if !actual.contains(id) {
            return false;
        }
        let row = &rows[i];
        let mask = masks.map_or(&Value::Null, |a| &a[i]);
        if super::service_references::scalar(row, mask, &["user_pool_config", "user_pool_id"])
            != Some(id)
        {
            return false;
        }
        let Some(region) = optional(row, mask, &["user_pool_config", "aws_region"]) else {
            return false;
        };
        let target_region = super::scope::scope(&nodes[s]).and_then(|s| s.region);
        let own_region = super::scope::scope(&nodes[r.target]).and_then(|s| s.region);
        if target_region.is_none()
            || region.map_or(own_region.as_ref() != target_region.as_ref(), |r| {
                target_region.as_ref().is_none_or(|s| s.0 != r)
            })
        {
            return false;
        }
        expected.insert(s);
    }
    !actual.is_empty()
        && expected.len() == actual.len()
        && expected == r.sources.iter().copied().collect()
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::appsync_datasource::annotate(nodes, refs, evidence);
    super::appsync_execution::annotate(nodes, refs, evidence);
    super::appsync_helpers::annotate(nodes, refs, evidence);
    super::appsync_events::annotate(nodes, refs, evidence);
    super::appsync_domains::annotate(nodes, refs, evidence);
    let configured: std::collections::BTreeSet<_> = refs
        .iter()
        .map(|r| (r.target, r.attribute.clone()))
        .collect();
    let extra: Vec<_> = refs
        .iter()
        .map(|r| {
            nodes[r.target].resource_type == API
                && r.attribute == ADDITIONAL
                && additional(nodes, refs, evidence, r)
        })
        .collect();
    for (r, extra) in refs.iter_mut().zip(extra) {
        let node = &nodes[r.target];
        if !matches!(node.resource_type.as_str(), API | ASSOCIATION) {
            continue;
        }
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        if r.attribute == ADDITIONAL {
            r.service_reference_verified = extra;
            continue;
        }
        let (kind, field) = match (node.resource_type.as_str(), r.attribute.as_str()) {
            (API, "lambda_authorizer_config.authorizer_uri") => ("aws_lambda_function", "arn"),
            (API, "user_pool_config.user_pool_id") => ("aws_cognito_user_pool", "id"),
            (API, "merged_api_execution_role_arn" | "log_config.cloudwatch_logs_role_arn") => {
                ("aws_iam_role", "arn")
            }
            (ASSOCIATION, "merged_api_id" | "source_api_id") => (API, "id"),
            (ASSOCIATION, "merged_api_arn" | "source_api_arn") => (API, "arn"),
            _ => continue,
        };
        let Some(s) = super::route53::endpoint(nodes, r, kind, &[field]) else {
            continue;
        };
        let endpoint = &nodes[s];
        if !super::route53::configured(node, endpoint) {
            continue;
        }
        let value = scalar(evidence, r.target, &r.attribute);
        if value.is_none() || value != scalar(evidence, s, field) {
            continue;
        }
        let left = super::scope::scope(node).and_then(|s| s.region);
        let right = super::scope::scope(endpoint).and_then(|s| s.region);
        if left.is_none() {
            continue;
        }
        if kind == "aws_cognito_user_pool" {
            let (v, m) = evidence[r.target];
            let Some(region) = optional(v, m, &["user_pool_config", "aws_region"]) else {
                continue;
            };
            if right.is_none()
                || region.map_or(left != right, |r| right.as_ref().is_none_or(|s| s.0 != r))
            {
                continue;
            }
        } else if kind != "aws_iam_role" && left != right {
            continue;
        }
        let discriminator = match r.attribute.as_str() {
            "lambda_authorizer_config.authorizer_uri" => {
                Some(("authentication_type", "AWS_LAMBDA"))
            }
            "user_pool_config.user_pool_id" => {
                Some(("authentication_type", "AMAZON_COGNITO_USER_POOLS"))
            }
            "merged_api_execution_role_arn" => Some(("api_type", "MERGED")),
            _ => None,
        };
        if discriminator.is_some_and(|(f, v)| scalar(evidence, r.target, f) != Some(v)) {
            continue;
        }
        if node.resource_type == ASSOCIATION {
            let other = if field == "id" {
                r.attribute.replace("_id", "_arn")
            } else {
                r.attribute.replace("_arn", "_id")
            };
            if configured.contains(&(r.target, other.clone()))
                || scalar(evidence, r.target, &other).is_some_and(|v| {
                    Some(v) != scalar(evidence, s, if field == "id" { "arn" } else { "id" })
                })
            {
                continue;
            }
            if r.attribute.starts_with("merged_")
                && scalar(evidence, s, "api_type") != Some("MERGED")
            {
                continue;
            }
            if r.attribute.starts_with("source_")
                && scalar(evidence, s, "api_type") != Some("GRAPHQL")
            {
                continue;
            }
            if field == "arn" {
                let parts: Vec<_> = value.unwrap().splitn(6, ':').collect();
                if parts.len() != 6
                    || parts[0] != "arn"
                    || parts[2] != "appsync"
                    || right.as_ref().is_none_or(|r| r.0 != parts[3])
                {
                    continue;
                }
                let Some(account) = crate::model::AwsAccountScope::new(parts[4]) else {
                    continue;
                };
                r.service_reference_owner_account = Some(account);
            }
        }
        r.service_reference_verified = true;
    }
    let pairs: Vec<_> = refs
        .iter()
        .filter(|r| nodes[r.target].resource_type == ASSOCIATION && r.service_reference_verified)
        .map(|r| (r.target, r.attribute.starts_with("merged_"), r.sources[0]))
        .collect();
    for r in refs
        .iter_mut()
        .filter(|r| nodes[r.target].resource_type == ASSOCIATION)
    {
        let merged: Vec<_> = pairs
            .iter()
            .filter(|(t, m, _)| *t == r.target && *m)
            .collect();
        let source: Vec<_> = pairs
            .iter()
            .filter(|(t, m, _)| *t == r.target && !*m)
            .collect();
        r.service_reference_verified &=
            merged.len() == 1 && source.len() == 1 && merged[0].2 != source[0].2;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    if super::appsync_domains::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some() {
        return super::appsync_domains::bindings(raw, r);
    }
    if super::appsync_events::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some() {
        return super::appsync_events::bindings(raw, r);
    }
    if super::appsync_helpers::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some() {
        return super::route53::bindings(raw, r);
    }
    if super::appsync_execution::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some() {
        return super::appsync_execution::bindings(raw, r);
    }
    let node = &raw.nodes[r.target];
    if node.resource_type == "aws_appsync_datasource" {
        return super::appsync_datasource::bindings(raw, r);
    }
    if raw.nodes[r.target].resource_type == ASSOCIATION {
        return endpoint_binding(raw, r)
            && raw
                .attributes
                .iter()
                .filter(|peer| peer.target == r.target && peer.service_reference_verified)
                .all(|peer| endpoint_binding(raw, peer));
    }
    super::route53::bindings(raw, r)
}

fn endpoint_binding(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    if super::route53::bindings(raw, r) {
        return r
            .service_reference_owner_account
            .as_ref()
            .is_none_or(|owner| {
                r.sources.iter().all(|&s| {
                    raw.nodes[s]
                        .provider_configuration
                        .as_ref()
                        .and_then(|p| p.account.as_ref())
                        .is_none_or(|account| account == owner)
                })
            });
    }
    if node.resource_type == ASSOCIATION && r.service_reference_owner_account.is_some() {
        return r.sources.iter().all(|&s| {
            let (Some(a), Some(b)) = (
                &node.provider_configuration,
                &raw.nodes[s].provider_configuration,
            ) else {
                return false;
            };
            !a.account_conflict
                && !b.account_conflict
                && a.account.is_some()
                && b.account == r.service_reference_owner_account
        });
    }
    super::route53::bindings(raw, r)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync.rs"]
mod tests;
