use super::address::{contextualize, static_address};
use super::references::{
    instance_map, qualify, qualify_reference, references, resolve_sources as resolve,
};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type ExpressionPaths = BTreeMap<String, (BTreeSet<String>, bool)>;
type IndexedExpressionPaths = BTreeMap<String, (BTreeSet<String>, bool, Vec<usize>)>;

#[derive(Default)]
struct ResourceExpressions {
    attributes: ExpressionPaths,
    indexed: IndexedExpressionPaths,
}

#[cfg(test)]
#[path = "../../tests/unit/plan/attributes.rs"]
mod tests;

pub(super) fn collect(
    module: &Value,
    nodes: &[TerraformEntity],
    symbols: &BTreeMap<String, BTreeSet<String>>,
    graph_only: bool,
) -> Vec<AttributeReference> {
    let mut expressions = BTreeMap::new();
    let indexed_resources = nodes
        .iter()
        .filter(|node| {
            crate::provider::has_indexed_service_references(&node.provider, &node.resource_type)
        })
        .map(|node| static_address(&node.address))
        .collect();
    collect_expressions(
        module,
        "",
        &BTreeMap::new(),
        &mut expressions,
        graph_only,
        &indexed_resources,
    );
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
            let ordinary = attributes
                .attributes
                .iter()
                .map(|(path, (refs, complete))| (path, refs, complete, &[][..]));
            let indexed = attributes
                .indexed
                .iter()
                .filter(|(path, _)| {
                    crate::provider::indexed_service_reference(
                        &node.provider,
                        &node.resource_type,
                        path,
                    )
                })
                .map(|(path, (refs, complete, lengths))| {
                    (path, refs, complete, lengths.as_slice())
                });
            for (attribute, refs, structurally_complete, lengths) in ordinary.chain(indexed) {
                let refs = refs
                    .iter()
                    .map(|r| contextualize(r, &node.address))
                    .collect();
                let resolution = resolve(&refs, &instances, &aliases);
                let source_attributes = resolution
                    .sources
                    .iter()
                    .flat_map(|&source| {
                        refs.iter().filter_map(move |reference: &String| {
                            reference
                                .strip_prefix(&format!("{}.", nodes[source].address))
                                .map(str::to_owned)
                        })
                    })
                    .collect();
                result.push(AttributeReference {
                    indexed_block_lengths: lengths.to_vec(),
                    target,
                    attribute: attribute.clone(),
                    has_references: !refs.is_empty(),
                    sources: resolution.sources,
                    complete: resolution.complete && *structurally_complete,
                    instances_explicit: resolution.instances_explicit,
                    collection_ids_complete: false,
                    scalar_id_matches: false,
                    scalar_name_matches: false,
                    arn_values_match: false,
                    messaging_endpoint_verified: false,
                    monitoring_verified: false,
                    source_attributes,
                    service_reference_verified: false,
                    service_reference_owner_account: None,
                    ids_only: resolution.ids_only,
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
    found: &mut BTreeMap<String, ResourceExpressions>,
    graph_only: bool,
    indexed_resources: &BTreeSet<String>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let address = qualify(scope, address);
            let include_indexed = indexed_resources.contains(&address);
            let resource_expressions = found.entry(address).or_default();
            let attributes = &mut resource_expressions.attributes;
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
                if include_indexed {
                    resource_expressions.indexed.extend(
                        indexed_paths(expression, name, &[]).into_iter().map(
                            |(path, (refs, complete, lengths))| {
                                (
                                    path,
                                    (
                                        refs.into_iter()
                                            .map(|r| qualify_reference(scope, &r))
                                            .collect(),
                                        complete,
                                        lengths,
                                    ),
                                )
                            },
                        ),
                    );
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
            collect_expressions(
                &call["module"],
                &child,
                &inherited,
                found,
                graph_only,
                indexed_resources,
            );
        }
    }
}

fn indexed_paths(value: &Value, path: &str, lengths: &[usize]) -> IndexedExpressionPaths {
    match value {
        Value::Object(fields) if super::references::expression_object(fields) => {
            nested_paths(value, path)
                .into_iter()
                .map(|(path, (refs, complete))| (path, (refs, complete, lengths.to_vec())))
                .collect()
        }
        Value::Object(fields) => fields
            .iter()
            .flat_map(|(name, value)| indexed_paths(value, &format!("{path}.{name}"), lengths))
            .collect(),
        Value::Array(values) => {
            let mut nested = lengths.to_vec();
            nested.push(values.len());
            values
                .iter()
                .enumerate()
                .flat_map(|(index, value)| {
                    indexed_paths(value, &format!("{path}.{index}"), &nested)
                })
                .collect()
        }
        _ => BTreeMap::new(),
    }
}

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
