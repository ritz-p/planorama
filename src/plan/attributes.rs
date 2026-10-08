use super::address::{contextualize, static_address};
use super::references::{
    instance_map, qualify, qualify_reference, references, resolve_sources as resolve,
};
use crate::model::{AttributeReference, Node};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type ExpressionPaths = BTreeMap<String, (BTreeSet<String>, bool)>;

#[cfg(test)]
#[path = "../../tests/unit/plan/attributes.rs"]
mod tests;

pub(super) fn collect(
    module: &Value,
    nodes: &[Node],
    symbols: &BTreeMap<String, BTreeSet<String>>,
    graph_only: bool,
) -> Vec<AttributeReference> {
    let mut expressions = BTreeMap::new();
    collect_expressions(module, "", &BTreeMap::new(), &mut expressions, graph_only);
    let aliases = symbols
        .iter()
        .filter(|(key, _)| !expressions.contains_key(*key))
        .map(|(key, references)| (key.clone(), references.clone()))
        .collect();
    let instances = instance_map(nodes);
    let mut result = Vec::new();
    for (target, node) in nodes.iter().enumerate() {
        if node.deposed_key.is_some() {
            continue;
        }
        if let Some(attributes) = expressions.get(&static_address(&node.address)) {
            for (attribute, (refs, structurally_complete)) in attributes {
                let refs = refs
                    .iter()
                    .map(|r| contextualize(r, &node.address))
                    .collect();
                let resolution = resolve(&refs, &instances, &aliases);
                result.push(AttributeReference {
                    target,
                    attribute: attribute.clone(),
                    sources: resolution.sources,
                    complete: resolution.complete && *structurally_complete,
                    collection_ids_complete: false,
                    issues: resolution.issues,
                });
            }
        }
    }
    result
}

fn collect_expressions(
    module: &Value,
    scope: &str,
    inherited: &BTreeMap<String, BTreeSet<String>>,
    found: &mut BTreeMap<String, ExpressionPaths>,
    graph_only: bool,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let attributes = found.entry(qualify(scope, address)).or_default();
            if graph_only {
                attributes.extend(
                    inherited
                        .iter()
                        .map(|(name, refs)| (name.clone(), (refs.clone(), true))),
                );
                attributes.extend(
                    graph_expressions(resource, scope)
                        .into_iter()
                        .map(|(name, refs)| (name, (refs, true))),
                );
            }
            for (name, expression) in resource["expressions"].as_object().into_iter().flatten() {
                if graph_only {
                    continue;
                }
                let mut refs = BTreeSet::new();
                references(expression, &mut refs);
                attributes.insert(
                    name.clone(),
                    (
                        refs.into_iter()
                            .map(|r| qualify_reference(scope, &r))
                            .collect(),
                        true,
                    ),
                );
                for (path, (refs, complete)) in nested_paths(expression, name) {
                    if path != *name {
                        attributes.insert(
                            path,
                            (
                                refs.into_iter()
                                    .map(|r| qualify_reference(scope, &r))
                                    .collect(),
                                complete,
                            ),
                        );
                    }
                }
            }
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            let child = qualify(scope, &format!("module.{name}"));
            let mut inherited = inherited.clone();
            inherited.extend(
                graph_expressions(call, scope)
                    .into_iter()
                    .map(|(attribute, refs)| (format!("{child}.{attribute}"), refs)),
            );
            collect_expressions(&call["module"], &child, &inherited, found, graph_only);
        }
    }
}

// These references participate in dependency routing but live outside the
// resource's attribute expressions in Terraform's configuration JSON.
fn graph_expressions(value: &Value, scope: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut result = BTreeMap::new();
    for name in ["depends_on", "count_expression", "for_each_expression"] {
        let mut refs = BTreeSet::new();
        if name == "depends_on" {
            refs.extend(
                value[name]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
        } else {
            references(&value[name], &mut refs);
        }
        if !refs.is_empty() {
            result.insert(
                name.into(),
                refs.into_iter()
                    .map(|r| qualify_reference(scope, &r))
                    .collect(),
            );
        }
    }
    result
}

fn nested_paths(value: &Value, path: &str) -> ExpressionPaths {
    match value {
        // Expression objects are leaves. Never walk literal values or expression metadata.
        Value::Object(fields) if super::references::expression_object(fields) => {
            let mut refs = BTreeSet::new();
            references(value, &mut refs);
            let complete = !fields.contains_key("constant_value")
                && fields
                    .get("references")
                    .and_then(Value::as_array)
                    .is_some_and(|refs| !refs.is_empty() && refs.iter().all(Value::is_string));
            BTreeMap::from([(path.into(), (refs, complete))])
        }
        Value::Object(fields) => block_paths(fields, path),
        Value::Array(blocks) => {
            let blocks: Vec<_> = blocks
                .iter()
                // Array entries are block field maps, even when a provider
                // names a field "references" or "constant_value".
                .map(|block| match block {
                    Value::Object(fields) => block_paths(fields, path),
                    _ => BTreeMap::new(),
                })
                .collect();
            let paths: BTreeSet<_> = blocks
                .iter()
                .flat_map(|block| block.keys().cloned())
                .collect();
            paths
                .into_iter()
                .map(|path| {
                    let mut refs = BTreeSet::new();
                    let mut complete = true;
                    for block in &blocks {
                        match block.get(&path) {
                            Some((part, valid)) => {
                                refs.extend(part.iter().cloned());
                                complete &= valid;
                            }
                            None => complete = false,
                        }
                    }
                    (path, (refs, complete))
                })
                .collect()
        }
        _ => BTreeMap::new(),
    }
}

fn block_paths(fields: &serde_json::Map<String, Value>, path: &str) -> ExpressionPaths {
    fields
        .iter()
        .flat_map(|(name, value)| nested_paths(value, &format!("{path}.{name}")))
        .collect()
}
