use super::address::static_address;
use super::references::{prefix_match, qualify, references};
use crate::model::{AttributeReference, Node};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/unit/plan/attributes.rs"]
mod tests;

pub(super) fn collect(
    module: &Value,
    nodes: &[Node],
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<AttributeReference> {
    let mut expressions = BTreeMap::new();
    collect_expressions(module, "", &mut expressions);
    let mut instances: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        instances
            .entry(static_address(&node.address))
            .or_default()
            .push(index);
    }
    let mut result = Vec::new();
    for (target, node) in nodes.iter().enumerate() {
        if let Some(attributes) = expressions.get(&static_address(&node.address)) {
            for (attribute, refs) in attributes {
                let (sources, complete) = resolve(refs, &instances, symbols);
                result.push(AttributeReference {
                    target,
                    attribute: attribute.clone(),
                    sources,
                    complete,
                });
            }
        }
    }
    result
}

fn collect_expressions(
    module: &Value,
    scope: &str,
    found: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let (Some(address), Some(expressions)) = (
            resource["address"].as_str(),
            resource["expressions"].as_object(),
        ) {
            let attributes = found.entry(qualify(scope, address)).or_default();
            for (name, expression) in expressions {
                let mut refs = BTreeSet::new();
                references(expression, &mut refs);
                attributes.insert(
                    name.clone(),
                    refs.into_iter().map(|r| qualify(scope, &r)).collect(),
                );
            }
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            collect_expressions(
                &call["module"],
                &qualify(scope, &format!("module.{name}")),
                found,
            );
        }
    }
}

fn resolve(
    refs: &BTreeSet<String>,
    instances: &BTreeMap<String, Vec<usize>>,
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> (Vec<usize>, bool) {
    let mut pending: Vec<_> = refs.iter().cloned().collect();
    let mut visited = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut complete = !pending.is_empty();
    while let Some(reference) = pending.pop() {
        let reference = static_address(&reference);
        if !visited.insert(reference.clone()) {
            continue;
        }
        match instances
            .iter()
            .filter(|(key, _)| prefix_match(&reference, key))
            .max_by_key(|(key, _)| key.len())
        {
            Some((_, indices)) => sources.extend(indices.iter().copied()),
            None => match symbols
                .iter()
                .filter(|(key, _)| prefix_match(&reference, key))
                .max_by_key(|(key, _)| key.len())
            {
                Some((_, aliases)) if !aliases.is_empty() => {
                    pending.extend(aliases.iter().cloned())
                }
                _ => complete = false,
            },
        }
    }
    (sources.into_iter().collect(), complete)
}
