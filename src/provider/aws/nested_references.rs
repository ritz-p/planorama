use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

type Row<'a> = (String, Vec<usize>, &'a Value);
fn rows<'a>(
    value: &'a Value,
    mask: &Value,
    path: &[&str],
    prefix: String,
    lengths: Vec<usize>,
    result: &mut Vec<Row<'a>>,
) -> Option<()> {
    if let Some(values) = value.as_array().filter(|_| !path.is_empty()) {
        if !matches!(
            prefix.rsplit('.').next(),
            Some(
                "replica"
                    | "rule"
                    | "destination"
                    | "target_groups"
                    | "origin"
                    | "cache_behavior"
                    | "endpoint_configuration"
                    | "resource"
                    | "ordered_cache_behavior"
                    | "function_association"
                    | "lambda_function_association"
            )
        ) && values.len() != 1
        {
            return None;
        }
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Array(_))
            || mask.as_array().is_some_and(|m| m.len() != values.len())
        {
            return None;
        }
        for (i, value) in values.iter().enumerate() {
            let mut lengths = lengths.clone();
            lengths.push(values.len());
            rows(
                value,
                &mask[i],
                path,
                format!("{prefix}.{i}"),
                lengths,
                result,
            )?;
        }
    } else if let Some((field, rest)) = path.split_first() {
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        if value[*field].is_null() {
            return matches!(&mask[*field], Value::Null | Value::Bool(false)).then_some(());
        }
        let prefix = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        rows(&value[*field], &mask[*field], rest, prefix, lengths, result)?;
    } else {
        if !known_leaf(value, mask) {
            return None;
        }
        result.push((prefix, lengths, value));
    }
    Some(())
}

pub(super) fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    fields: fn(&str, &str, &str) -> &'static [&'static str],
    valid: impl Fn(usize, &str) -> bool,
) -> Option<()> {
    let node = &nodes[r.target];
    if !current(node) || !r.issues.is_empty() || !r.instances_explicit {
        return None;
    }
    if refs
        .iter()
        .filter(|other| other.target == r.target && other.attribute == r.attribute)
        .count()
        != 1
    {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let mut leaves = Vec::new();
    rows(
        value,
        mask,
        &r.attribute.split('.').collect::<Vec<_>>(),
        String::new(),
        Vec::new(),
        &mut leaves,
    )?;
    if leaves.is_empty() {
        return None;
    }
    let mut sources = BTreeSet::new();
    for (path, lengths, actual) in leaves {
        let reference = if lengths.is_empty() {
            r
        } else {
            let mut found = refs
                .iter()
                .filter(|s| s.target == r.target && s.attribute == path);
            let reference = found.next()?;
            if found.next().is_some() || reference.indexed_block_lengths != lengths {
                return None;
            }
            reference
        };
        if !reference.complete || !reference.instances_explicit || !reference.issues.is_empty() {
            return None;
        }
        let actual: BTreeSet<_> = if let Some(values) = actual.as_array() {
            let values: BTreeSet<_> = values.iter().map(Value::as_str).collect::<Option<_>>()?;
            if values.len() != actual.as_array()?.len() {
                return None;
            }
            values
        } else {
            [actual.as_str()?].into()
        };
        if actual.is_empty() || actual.contains("") {
            return None;
        }
        let mut expected = BTreeSet::new();
        for &source in &reference.sources {
            let endpoint = &nodes[source];
            if source == r.target || !current(endpoint) || !valid(source, &path) {
                return None;
            }
            let outputs = fields(
                &node.resource_type,
                &r.attribute,
                super::aliases::entity_type(endpoint),
            );
            let attrs = reference.source_attributes_by_instance.get(&source)?;
            let outputs_by_instance = attrs.iter().collect::<Vec<_>>();
            let [output] = outputs_by_instance.as_slice() else {
                return None;
            };
            if !outputs.contains(&output.as_str()) {
                return None;
            }
            let expected_value = super::service_evidence::known(evidence, source, output)?;
            if !expected.insert(expected_value) {
                return None;
            }
            sources.insert(source);
        }
        if actual != expected {
            return None;
        }
    }
    (sources == r.sources.iter().copied().collect()).then_some(())
}

pub(super) fn value<'a>(mut value: &'a Value, mut mask: &Value, path: &str) -> Option<&'a Value> {
    for field in path.split('.') {
        if let Ok(index) = field.parse::<usize>() {
            let values = value.as_array()?;
            if !matches!(mask, Value::Null | Value::Bool(false) | Value::Array(_))
                || mask.as_array().is_some_and(|m| m.len() != values.len())
            {
                return None;
            }
            value = values.get(index)?;
            mask = &mask[index];
        } else {
            if !value.is_object()
                || !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
            {
                return None;
            }
            value = &value[field];
            mask = &mask[field];
        }
    }
    known_leaf(value, mask).then_some(value)
}

fn known_leaf(value: &Value, mask: &Value) -> bool {
    matches!(mask, Value::Null | Value::Bool(false))
        || value
            .as_array()
            .zip(mask.as_array())
            .is_some_and(|(values, masks)| {
                values.len() == masks.len()
                    && masks
                        .iter()
                        .all(|m| matches!(m, Value::Null | Value::Bool(false)))
            })
}
