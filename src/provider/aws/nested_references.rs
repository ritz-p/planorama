use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn current(node: &TerraformEntity, mode: EntityMode) -> bool {
    node.provider.is_aws()
        && node.mode == mode
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
    collections: &[&str],
) -> Option<()> {
    if let Some(values) = value.as_array().filter(|_| !path.is_empty()) {
        if !matches!(
            prefix.rsplit('.').next(),
            Some(
                "replica"
                    | "rule"
                    | "destination"
                    | "target_groups"
                    | "listener"
                    | "origin"
                    | "cache_behavior"
                    | "endpoint_configuration"
                    | "resource"
                    | "ordered_cache_behavior"
                    | "function_association"
                    | "lambda_function_association"
                    | "representations"
                    | "jdbc_target"
                    | "mongodb_target"
                    | "s3_target"
                    | "catalog_target"
                    | "delta_target"
                    | "hudi_target"
                    | "iceberg_target"
                    | "dynamodb_target"
                    | "input_record_tables"
                    | "actions"
                    | "conditions"
            )
        ) && !collections.contains(
            &prefix
                .split('.')
                .filter(|s| s.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join(".")
                .as_str(),
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
                collections,
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
        rows(
            &value[*field],
            &mask[*field],
            rest,
            prefix,
            lengths,
            result,
            collections,
        )?;
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
    verify_collections(nodes, refs, evidence, r, fields, valid, &[])
}

pub(super) fn leaf_values<'a>(_: &str, actual: &'a Value) -> Option<BTreeSet<&'a str>> {
    Some(if let Some(values) = actual.as_array() {
        let values: BTreeSet<_> = values.iter().map(Value::as_str).collect::<Option<_>>()?;
        if values.len() != actual.as_array()?.len() {
            return None;
        }
        values
    } else {
        [actual.as_str()?].into()
    })
}

pub(super) fn verify_collections(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    fields: fn(&str, &str, &str) -> &'static [&'static str],
    valid: impl Fn(usize, &str) -> bool,
    collections: &[&str],
) -> Option<()> {
    verify_values(
        nodes,
        refs,
        evidence,
        r,
        fields,
        valid,
        (
            NestedBlocks::Indexed(collections),
            leaf_values,
            EntityMode::Managed,
        ),
    )
}

#[derive(Clone, Copy)]
pub(super) enum NestedBlocks<'a> {
    Indexed(&'a [&'a str]),
    ScalarSet(Option<&'a str>),
    Unordered(&'a [&'a str]),
    WafStatement,
}

fn endpoint_value<'a>(
    reference: &AttributeReference,
    source: usize,
    evidence: &[(&'a Value, &Value)],
    outputs: &[&str],
) -> Option<&'a str> {
    let attrs = reference.source_attributes_by_instance.get(&source)?;
    if outputs.contains(&"roots[].id") {
        let mut indices = attrs
            .iter()
            .filter_map(|p| super::organizations::root_index(p));
        let index = indices.next()?;
        if indices.next().is_some()
            || !attrs.iter().all(|p| {
                super::organizations::root_index(p) == Some(index)
                    || p == "roots"
                    || p == &format!("roots[{index}]")
            })
        {
            return None;
        }
        let (v, m) = evidence[source];
        return value(v, m, &format!("roots.{index}.id"))?.as_str();
    }
    let output = attrs.first()?;
    if attrs.len() != 1 || !outputs.contains(&output.as_str()) {
        return None;
    }
    super::service_evidence::known(evidence, source, output)
}

fn verify_scalar_set(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    fields: impl Fn(&str, &str, &str) -> &'static [&'static str],
    valid: impl Fn(usize, &str) -> bool,
    (set_path, mode): (Option<&str>, EntityMode),
) -> Option<()> {
    let (root, _) = r.attribute.split_once('.')?;
    let set_path = set_path.unwrap_or(root);
    let field = r
        .attribute
        .strip_prefix(set_path)?
        .strip_prefix('.')?
        .replace('.', ".0.");
    let leaf_fields = r.attribute.split('.').collect::<Vec<_>>();
    let set_index = set_path.split('.').count() - 1;
    let set_location = set_path.replace('.', ".0.");
    let (v, mask) = evidence[r.target];
    let count = block_len(v, mask, &set_location)?;
    let mut config_lengths = vec![1; leaf_fields.len() - 1];
    config_lengths[set_index] = count;
    let path = |index| format!("{set_location}.{index}.{field}");
    let mut expected = BTreeMap::new();
    for index in 0..count {
        let attribute = path(index);
        let mut matches = refs
            .iter()
            .filter(|leaf| leaf.target == r.target && leaf.attribute == attribute);
        let Some(leaf) = matches.next() else { continue };
        if matches.next().is_some() || leaf.indexed_block_lengths != config_lengths {
            return None;
        }
        if leaf.root_constant_null {
            continue;
        }
        let [source] = *leaf.sources.as_slice() else {
            return None;
        };
        let endpoint = &nodes[source];
        if !leaf.complete
            || !leaf.instances_explicit
            || !leaf.issues.is_empty()
            || source == r.target
            || !current(endpoint, mode)
        {
            return None;
        }
        let outputs = fields(
            &nodes[r.target].resource_type,
            &r.attribute,
            super::aliases::entity_type(endpoint),
        );
        let id = endpoint_value(leaf, source, evidence, outputs)?;
        let (owner, occurrences) = expected.entry(id).or_insert((source, 0usize));
        if *owner != source {
            return None;
        }
        *occurrences += 1;
    }
    if expected.is_empty()
        || expected.values().map(|(s, _)| *s).collect::<BTreeSet<_>>()
            != r.sources.iter().copied().collect()
    {
        return None;
    }
    let mut observed = Vec::new();
    rows(
        v,
        mask,
        &leaf_fields,
        String::new(),
        Vec::new(),
        &mut observed,
        &[set_path],
    )?;
    for (attribute, lengths, actual) in observed {
        if lengths.len() != leaf_fields.len() - 1 {
            return None;
        }
        let (source, remaining) = expected.get_mut(actual.as_str()?)?;
        *remaining = remaining.checked_sub(1)?;
        if !valid(*source, &attribute) {
            return None;
        }
    }
    expected.values().all(|(_, n)| *n == 0).then_some(())
}

fn verify_unordered(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    fields: impl Fn(&str, &str, &str) -> &'static [&'static str],
    valid: impl Fn(usize, &str) -> bool,
    (collections, mode): (&[&str], EntityMode),
) -> Option<()> {
    let leaf_fields: Vec<_> = r.attribute.split('.').collect();
    let depth = leaf_fields.len() - 1;
    let mut expected = BTreeMap::new();
    for leaf in refs.iter().filter(|leaf| {
        leaf.target == r.target
            && leaf.indexed_block_lengths.len() == depth
            && leaf
                .attribute
                .split('.')
                .filter(|part| part.parse::<usize>().is_err())
                .eq(leaf_fields.iter().copied())
    }) {
        if !leaf.complete || !leaf.instances_explicit || !leaf.issues.is_empty() {
            return None;
        }
        if leaf.sources.is_empty() && !leaf.has_references {
            continue;
        }
        let [source] = *leaf.sources.as_slice() else {
            return None;
        };
        let endpoint = &nodes[source];
        if source == r.target || !current(endpoint, mode) {
            return None;
        }
        let outputs = fields(
            &nodes[r.target].resource_type,
            &r.attribute,
            super::aliases::entity_type(endpoint),
        );
        let id = endpoint_value(leaf, source, evidence, outputs)?;
        let (owner, counts) = expected.entry(id).or_insert((source, BTreeMap::new()));
        if *owner != source {
            return None;
        }
        *counts
            .entry(leaf.indexed_block_lengths.clone())
            .or_insert(0usize) += 1;
    }
    if expected.is_empty()
        || expected.values().map(|(s, _)| *s).collect::<BTreeSet<_>>()
            != r.sources.iter().copied().collect()
    {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let mut observed = Vec::new();
    rows(
        value,
        mask,
        &leaf_fields,
        String::new(),
        Vec::new(),
        &mut observed,
        collections,
    )?;
    for (path, lengths, value) in observed {
        if lengths.len() != depth {
            return None;
        }
        if let Some((source, counts)) = expected.get_mut(value.as_str()?) {
            let count = counts.get_mut(&lengths)?;
            *count = count.checked_sub(1)?;
            if !valid(*source, &path) {
                return None;
            }
        }
    }
    expected
        .values()
        .all(|(_, counts)| counts.values().all(|&count| count == 0))
        .then_some(())
}

pub(super) type DecodeLeaf = for<'a> fn(&str, &'a Value) -> Option<BTreeSet<&'a str>>;

pub(super) fn verify_values(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    fields: impl Fn(&str, &str, &str) -> &'static [&'static str],
    valid: impl Fn(usize, &str) -> bool,
    (blocks, decode, endpoint_mode): (NestedBlocks, DecodeLeaf, EntityMode),
) -> Option<()> {
    let node = &nodes[r.target];
    if !current(node, EntityMode::Managed) || !r.issues.is_empty() || !r.instances_explicit {
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
    let repeated;
    let collections = match blocks {
        NestedBlocks::Indexed(paths) => paths,
        NestedBlocks::ScalarSet(path) => {
            return verify_scalar_set(
                nodes,
                refs,
                evidence,
                r,
                fields,
                valid,
                (path, endpoint_mode),
            );
        }
        NestedBlocks::Unordered(paths) => {
            return verify_unordered(
                nodes,
                refs,
                evidence,
                r,
                fields,
                valid,
                (paths, endpoint_mode),
            );
        }
        NestedBlocks::WafStatement => {
            repeated = r
                .attribute
                .match_indices(".statement.")
                .filter_map(|(i, _)| {
                    let path = &r.attribute[..i];
                    (path.ends_with(".and_statement") || path.ends_with(".or_statement"))
                        .then_some(&r.attribute[..i + ".statement".len()])
                })
                .collect::<Vec<_>>();
            &repeated
        }
    };
    let (value, mask) = evidence[r.target];
    let mut leaves = Vec::new();
    rows(
        value,
        mask,
        &r.attribute.split('.').collect::<Vec<_>>(),
        String::new(),
        Vec::new(),
        &mut leaves,
        collections,
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
        let actual = decode(&r.attribute, actual)?;
        if actual.is_empty() || actual.contains("") {
            return None;
        }
        let mut expected = BTreeSet::new();
        for &source in &reference.sources {
            let endpoint = &nodes[source];
            if source == r.target || !current(endpoint, endpoint_mode) || !valid(source, &path) {
                return None;
            }
            let outputs = fields(
                &node.resource_type,
                &r.attribute,
                super::aliases::entity_type(endpoint),
            );
            let expected_value = endpoint_value(reference, source, evidence, outputs)?;
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

fn at<'a, 'b>(
    mut value: &'a Value,
    mut mask: &'b Value,
    path: &str,
) -> Option<(&'a Value, &'b Value)> {
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
    Some((value, mask))
}

pub(super) fn value<'a>(value: &'a Value, mask: &Value, path: &str) -> Option<&'a Value> {
    let (value, mask) = at(value, mask, path)?;
    known_leaf(value, mask).then_some(value)
}

pub(super) fn singleton(value: &Value, mask: &Value, path: &str) -> bool {
    block(value, mask, path, 1)
}

pub(super) fn block(value: &Value, mask: &Value, path: &str, maximum: usize) -> bool {
    block_len(value, mask, path).is_some_and(|length| length <= maximum)
}

pub(super) fn block_len(value: &Value, mask: &Value, path: &str) -> Option<usize> {
    let (value, mask) = at(value, mask, path)?;
    let values = value.as_array()?;
    (!values.is_empty()
        && values.iter().all(Value::is_object)
        && (matches!(mask, Value::Null | Value::Bool(false))
            || mask.as_array().is_some_and(|masks| {
                masks.len() == values.len()
                    && masks.iter().all(|mask| {
                        matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
                    })
            })))
    .then_some(values.len())
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
