use crate::model::{Action, AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn current(node: &TerraformEntity) -> bool {
    node.provider.is_cloudflare()
        && super::known(&node.resource_type, node.mode)
        && node.deposed_key.is_none()
        && node.action != Action::Delete
        && node.metadata.state_removal.is_none()
}

pub(super) fn at<'a, 'b>(
    mut value: &'a Value,
    mut mask: &'b Value,
    path: &str,
) -> Option<(&'a Value, &'b Value)> {
    for field in path.split('.') {
        if !value.is_object()
            || !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        value = &value[field];
        mask = &mask[field];
    }
    Some((value, mask))
}

pub(super) fn text<'a>(value: &'a Value, mask: &Value, path: &str) -> Option<&'a str> {
    let (value, mask) = at(value, mask, path)?;
    if !matches!(mask, Value::Null | Value::Bool(false)) {
        return None;
    }
    value.as_str().filter(|s| !s.is_empty())
}

pub(super) fn rows<'a, 'b>(
    value: &'a Value,
    mask: &'b Value,
) -> Option<Vec<(&'a Value, &'b Value)>> {
    let values = value.as_array()?;
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Array(_))
        || mask.as_array().is_some_and(|m| m.len() != values.len())
    {
        return None;
    }
    Some(
        values
            .iter()
            .enumerate()
            .map(|(i, v)| (v, &mask[i]))
            .collect(),
    )
}

pub(super) fn unmasked(mask: &Value) -> bool {
    match mask {
        Value::Null | Value::Bool(false) => true,
        Value::Object(fields) => fields.values().all(unmasked),
        Value::Array(values) => values.iter().all(unmasked),
        _ => false,
    }
}

pub(super) fn list<'a>(value: &'a Value, mask: &Value) -> Option<Vec<&'a str>> {
    let result: Vec<_> = rows(value, mask)?
        .into_iter()
        .map(|(v, m)| {
            matches!(m, Value::Null | Value::Bool(false))
                .then(|| v.as_str().filter(|s| !s.is_empty()))
                .flatten()
        })
        .collect::<Option<_>>()?;
    (result.iter().copied().collect::<BTreeSet<_>>().len() == result.len()).then_some(result)
}

pub(super) fn same_account(a: &TerraformEntity, b: &TerraformEntity) -> bool {
    a.cloudflare_scope
        .as_ref()
        .and_then(|s| s.account.as_ref())
        .is_some_and(|account| {
            b.cloudflare_scope.as_ref().and_then(|s| s.account.as_ref()) == Some(account)
        })
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
    evidence: &[(&Value, &Value)],
    targets: &[Target<'_>],
    compatible: impl Fn(&TerraformEntity, &TerraformEntity) -> bool,
) -> bool {
    let consumer = &nodes[r.target];
    if !current(consumer)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || targets.is_empty()
        || refs
            .iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            != 1
    {
        return false;
    }
    let mut sources = BTreeSet::new();
    for target in targets {
        let mut candidates = nodes.iter().enumerate().filter(|(index, node)| {
            let (v, m) = evidence[*index];
            current(node)
                && node.resource_type == target.kind
                && compatible(consumer, node)
                && text(v, m, target.output) == Some(target.value)
        });
        let Some((source, _)) = candidates.next() else {
            return false;
        };
        if candidates.next().is_some() || source == r.target || !r.sources.contains(&source) {
            return false;
        }
        let Some(attributes) = r.source_attributes_by_instance.get(&source) else {
            return false;
        };
        if attributes.len() != 1 || !attributes.contains(target.output) {
            return false;
        }
        sources.insert(source);
    }
    sources == r.sources.iter().copied().collect()
}
