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
    let aliases = symbols
        .iter()
        .filter(|(key, _)| !expressions.contains_key(*key))
        .map(|(key, references)| (key.clone(), references.clone()))
        .collect();
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
                let (sources, complete) = resolve(refs, &instances, &aliases);
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
        if let Some(address) = resource["address"].as_str() {
            let attributes = found.entry(qualify(scope, address)).or_default();
            for (name, expression) in resource["expressions"].as_object().into_iter().flatten() {
                let mut refs = BTreeSet::new();
                references(expression, &mut refs);
                attributes.insert(
                    name.clone(),
                    refs.into_iter().map(|r| qualify(scope, &r)).collect(),
                );
                if name == "network_configuration" {
                    if let Some(refs) = subnet_references(expression) {
                        attributes.insert(
                            "network_configuration.subnets".into(),
                            refs.into_iter().map(|r| qualify(scope, &r)).collect(),
                        );
                    }
                }
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

fn subnet_references(block: &Value) -> Option<BTreeSet<String>> {
    match block {
        Value::Array(blocks) if !blocks.is_empty() => {
            let mut found = BTreeSet::new();
            for block in blocks {
                found.extend(subnet_references(block)?);
            }
            Some(found)
        }
        Value::Object(fields) => {
            let expression = fields.get("subnets")?;
            let refs = expression.get("references")?.as_array()?;
            if refs.is_empty() || expression.get("constant_value").is_some() {
                return None;
            }
            refs.iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect()
        }
        _ => None,
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
                None if reference.starts_with("module.") => {
                    let descendants: Vec<_> = instances
                        .iter()
                        .filter(|(key, _)| prefix_match(key, &reference))
                        .flat_map(|(_, indices)| indices.iter().copied())
                        .collect();
                    if descendants.is_empty() {
                        complete = false;
                    }
                    sources.extend(descendants);
                }
                _ => complete = false,
            },
        }
    }
    (sources.into_iter().collect(), complete)
}
