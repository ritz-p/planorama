use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/elbv2_cognito.rs"]
mod tests;

pub(crate) fn block(kind: &str) -> Option<&'static str> {
    match super::aliases::canonical(kind) {
        "aws_lb_listener" => Some("default_action"),
        "aws_lb_listener_rule" => Some("action"),
        _ => None,
    }
}

fn endpoint(field: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match field {
        "user_pool_arn" => Some(("aws_cognito_user_pool", "arn", "Authentication user pool")),
        "user_pool_client_id" => Some((
            "aws_cognito_user_pool_client",
            "id",
            "Authentication client",
        )),
        "user_pool_domain" => Some((
            "aws_cognito_user_pool_domain",
            "domain",
            "Authentication domain",
        )),
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, field: &str) -> bool {
    let parts: Vec<_> = field.split('.').collect();
    matches!(parts.as_slice(), [b, i, "authenticate_cognito", "0", f]
        if Some(*b) == block(kind) && i.parse::<usize>().is_ok() && endpoint(f).is_some())
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    let parts: Vec<_> = field.split('.').collect();
    match parts.as_slice() {
        [b, "authenticate_cognito", f] if Some(*b) == block(kind) => {
            endpoint(f).map(|(_, _, label)| label)
        }
        _ => None,
    }
}

fn scalar<'a>(value: &'a Value, mask: &Value, key: &str) -> Option<&'a str> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&mask[key], Value::Null | Value::Bool(false))
    {
        return None;
    }
    value[key].as_str().filter(|s| !s.is_empty())
}

fn rows<'a>(value: &'a Value, mask: &'a Value, key: &str) -> Option<Vec<(&'a Value, &'a Value)>> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let values = value[key].as_array().filter(|v| !v.is_empty())?;
    let masks = match &mask[key] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(m) if m.len() == values.len() => Some(m),
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

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let b = block(&nodes[r.target].resource_type)?;
    let field = r.attribute.rsplit('.').next()?;
    let (kind, output, _) = endpoint(field)?;
    if !r.issues.is_empty() {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let actions = rows(value, mask, b)?;
    let mut sources = BTreeSet::new();
    for (i, (action, mask)) in actions.iter().enumerate() {
        let action_type = scalar(action, mask, "type")?;
        let path = format!("{b}.{i}.authenticate_cognito.0.{field}");
        let mut found = refs
            .iter()
            .filter(|s| s.target == r.target && s.attribute == path);
        let reference = found.next();
        if action_type != "authenticate-cognito" {
            if reference.is_some()
                || !matches!(
                    &mask["authenticate_cognito"],
                    Value::Null | Value::Bool(false)
                )
                || !(action["authenticate_cognito"].is_null()
                    || action["authenticate_cognito"]
                        .as_array()
                        .is_some_and(Vec::is_empty))
            {
                return None;
            }
            continue;
        }
        let auth = rows(action, mask, "authenticate_cognito")?;
        let [(auth, auth_mask)] = auth.as_slice() else {
            return None;
        };
        let reference = reference?;
        let [source] = reference.sources.as_slice() else {
            return None;
        };
        if found.next().is_some()
            || reference.indexed_block_lengths != [actions.len(), 1]
            || !super::vpc_routes::eligible(nodes, reference)
            || nodes[*source].resource_type != kind
            || reference.source_attributes.len() != 1
            || !reference.source_attributes.contains(output)
        {
            return None;
        }
        let expected = super::service_evidence::known(evidence, *source, output)?;
        if scalar(auth, auth_mask, field) != Some(expected) {
            return None;
        }
        sources.insert(*source);
    }
    (!sources.is_empty() && sources == r.sources.iter().copied().collect()).then_some(())
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
                verify(nodes, refs, evidence, r).is_some()
            } else {
                r.service_reference_verified
            }
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        r.service_reference_verified = verified;
        if verified && label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.complete = true;
        }
    }
}
