use super::appsync::scalar;
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync_events.rs"]
mod tests;

const API: &str = "aws_appsync_api";
const NAMESPACE: &str = "aws_appsync_channel_namespace";
const COGNITO: &str = "event_config.auth_provider.cognito_config.user_pool_id";
const LAMBDA: &str = "event_config.auth_provider.lambda_authorizer_config.authorizer_uri";
const LOG: &str = "event_config.log_config.cloudwatch_logs_role_arn";
const PUBLISH: &str = "handler_configs.on_publish.integration.data_source_name";
const SUBSCRIBE: &str = "handler_configs.on_subscribe.integration.data_source_name";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (API, COGNITO) => Some("Event API Cognito user pool"),
        (API, LAMBDA) => Some("Event API Lambda authorizer"),
        (API, LOG) => Some("Event API logging role"),
        (NAMESPACE, "api_id") => Some("Namespace Event API"),
        (NAMESPACE, PUBLISH | SUBSCRIBE) => Some("API-qualified event data source"),
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == API
        && path != COGNITO
        && path != LAMBDA
        && matches!(
            path.split('.')
                .filter(|p| p.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join(".")
                .as_str(),
            COGNITO | LAMBDA
        )
}

fn rows<'a>(v: &'a Value, m: &'a Value, key: &str) -> Option<Vec<(&'a Value, &'a Value)>> {
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let values = v[key].as_array().filter(|v| !v.is_empty())?;
    let masks = match &m[key] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == values.len() => Some(a),
        _ => return None,
    };
    Some(
        values
            .iter()
            .enumerate()
            .map(|(i, v)| (v, masks.map_or(&Value::Null, |m| &m[i])))
            .collect(),
    )
}

fn compatible(nodes: &[TerraformEntity], a: usize, b: usize) -> bool {
    let region = super::scope::scope(&nodes[a]).and_then(|s| s.region);
    super::route53::configured(&nodes[a], &nodes[b])
        && region.is_some()
        && region == super::scope::scope(&nodes[b]).and_then(|s| s.region)
}

fn nested<'a>(evidence: &[(&'a Value, &'a Value)], index: usize, path: &str) -> Option<&'a str> {
    let parts: Vec<_> = path.split('.').collect();
    let (mut v, mut m) = evidence[index];
    for key in &parts[..parts.len() - 1] {
        let list = rows(v, m, key)?;
        let [row] = list.as_slice() else { return None };
        (v, m) = *row;
    }
    super::service_evidence::known(&[(v, m)], 0, parts.last()?)
}

fn reference<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    found.next().is_none().then_some(r)
}

fn api(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
) -> Option<usize> {
    let r = reference(refs, target, "api_id")?;
    let s = super::route53::endpoint(nodes, r, API, &["api_id"])?;
    (compatible(nodes, target, s)
        && scalar(evidence, target, "api_id").is_some()
        && scalar(evidence, target, "api_id") == scalar(evidence, s, "api_id"))
    .then_some(s)
}

fn auth(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if super::scope::scope(&nodes[r.target])
        .and_then(|s| s.region)
        .is_none()
    {
        return false;
    }
    let (v, m) = evidence[r.target];
    let Some(config) = rows(v, m, "event_config") else {
        return false;
    };
    let [(v, m)] = config.as_slice() else {
        return false;
    };
    let Some(providers) = rows(v, m, "auth_provider") else {
        return false;
    };
    let (typ, block, field, kind, output) = if r.attribute == COGNITO {
        (
            "AMAZON_COGNITO_USER_POOLS",
            "cognito_config",
            "user_pool_id",
            "aws_cognito_user_pool",
            "id",
        )
    } else {
        (
            "AWS_LAMBDA",
            "lambda_authorizer_config",
            "authorizer_uri",
            "aws_lambda_function",
            "arn",
        )
    };
    let mut expected = BTreeSet::new();
    for (i, (v, m)) in providers.iter().enumerate() {
        let Some(auth) = super::service_evidence::known(&[(*v, *m)], 0, "auth_type") else {
            return false;
        };
        if !matches!(
            auth,
            "API_KEY" | "AWS_IAM" | "AMAZON_COGNITO_USER_POOLS" | "AWS_LAMBDA" | "OPENID_CONNECT"
        ) {
            return false;
        }
        if auth != typ {
            if v.get(block)
                .is_some_and(|v| !v.is_null() && !v.as_array().is_some_and(Vec::is_empty))
                || reference(
                    refs,
                    r.target,
                    &format!("event_config.0.auth_provider.{i}.{block}.0.{field}"),
                )
                .is_some()
            {
                return false;
            }
            continue;
        }
        let Some(config) = rows(v, m, block) else {
            return false;
        };
        let [(v, m)] = config.as_slice() else {
            return false;
        };
        let path = format!("event_config.0.auth_provider.{i}.{block}.0.{field}");
        let Some(indexed) = reference(refs, r.target, &path) else {
            return false;
        };
        let Some(s) = super::route53::endpoint(nodes, indexed, kind, &[output]) else {
            return false;
        };
        if indexed.indexed_block_lengths != [1, providers.len(), 1]
            || !super::route53::configured(&nodes[r.target], &nodes[s])
            || super::service_evidence::known(&[(*v, *m)], 0, field).is_none()
            || super::service_evidence::known(&[(*v, *m)], 0, field) != scalar(evidence, s, output)
        {
            return false;
        }
        if r.attribute == COGNITO {
            let region = super::scope::scope(&nodes[s]).and_then(|s| s.region);
            if region.as_ref().is_none_or(|r| {
                Some(r.0.as_str()) != super::service_evidence::known(&[(*v, *m)], 0, "aws_region")
            }) {
                return false;
            }
        } else if !compatible(nodes, r.target, s) {
            return false;
        }
        expected.insert(s);
    }
    !expected.is_empty()
        && expected == r.sources.iter().copied().collect()
        && r.instances_explicit
        && r.issues.is_empty()
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
    let results: Vec<_> =
        refs.iter()
            .map(|r| {
                let kind = nodes[r.target].resource_type.as_str();
                label(kind, &r.attribute)?;
                if counts[&(r.target, r.attribute.clone())] != 1 {
                    return Some(false);
                }
                let result = match r.attribute.as_str() {
                    COGNITO | LAMBDA => auth(nodes, refs, evidence, r),
                    LOG => super::route53::endpoint(nodes, r, "aws_iam_role", &["arn"])
                        .is_some_and(|s| {
                            super::route53::configured(&nodes[r.target], &nodes[s])
                                && super::scope::scope(&nodes[r.target])
                                    .and_then(|s| s.region)
                                    .is_some()
                                && nested(evidence, r.target, LOG).is_some()
                                && nested(evidence, r.target, LOG) == scalar(evidence, s, "arn")
                        }),
                    "api_id" => api(nodes, refs, evidence, r.target).is_some(),
                    PUBLISH | SUBSCRIBE => {
                        super::route53::endpoint(nodes, r, "aws_appsync_datasource", &["name"])
                            .is_some_and(|s| {
                                let behavior = if r.attribute == PUBLISH {
                                    "handler_configs.on_publish.behavior"
                                } else {
                                    "handler_configs.on_subscribe.behavior"
                                };
                                compatible(nodes, r.target, s)
                                    && matches!(
                                        nested(evidence, r.target, behavior),
                                        Some("CODE" | "DIRECT")
                                    )
                                    && nested(evidence, r.target, &r.attribute).is_some()
                                    && nested(evidence, r.target, &r.attribute)
                                        == scalar(evidence, s, "name")
                                    && api(nodes, refs, evidence, r.target)
                                        .zip(api(nodes, refs, evidence, s))
                                        .is_some_and(|(a, b)| a == b)
                            })
                    }
                    _ => false,
                };
                Some(result)
            })
            .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(verified) = result {
            r.service_reference_verified = verified;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if !super::route53::bindings(raw, r) {
        return false;
    }
    if !matches!(r.attribute.as_str(), PUBLISH | SUBSCRIBE) {
        return true;
    }
    let [s] = r.sources.as_slice() else {
        return false;
    };
    let Some((a, b)) = reference(&raw.attributes, r.target, "api_id").zip(reference(
        &raw.attributes,
        *s,
        "api_id",
    )) else {
        return false;
    };
    a.service_reference_verified
        && a.sources == b.sources
        && super::route53::bindings(raw, a)
        && super::route53::bindings(raw, b)
}
