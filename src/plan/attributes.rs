use super::address::{contextualize, static_address};
use super::references::{
    instance_map, qualify, qualify_reference, references, resolve_sources as resolve,
};
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
        if let Some(attributes) = expressions.get(&static_address(&node.address)) {
            for (attribute, refs) in attributes {
                let refs = refs
                    .iter()
                    .map(|r| contextualize(r, &node.address))
                    .collect();
                let resolution = resolve(&refs, &instances, &aliases);
                result.push(AttributeReference {
                    target,
                    attribute: attribute.clone(),
                    sources: resolution.sources,
                    complete: resolution.complete,
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
    found: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    graph_only: bool,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let attributes = found.entry(qualify(scope, address)).or_default();
            if graph_only {
                attributes.extend(
                    inherited
                        .iter()
                        .map(|(name, refs)| (name.clone(), refs.clone())),
                );
                attributes.extend(graph_expressions(resource, scope));
            }
            for (name, expression) in resource["expressions"].as_object().into_iter().flatten() {
                if graph_only {
                    continue;
                }
                let mut refs = BTreeSet::new();
                references(expression, &mut refs);
                attributes.insert(
                    name.clone(),
                    refs.into_iter()
                        .map(|r| qualify_reference(scope, &r))
                        .collect(),
                );
                if name == "network_configuration" {
                    if let Some(refs) = subnet_references(expression) {
                        attributes.insert(
                            "network_configuration.subnets".into(),
                            refs.into_iter()
                                .map(|r| qualify_reference(scope, &r))
                                .collect(),
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
            if refs.iter().any(|value| !value.is_string()) {
                return None;
            }
            let mut found = BTreeSet::new();
            references(expression, &mut found);
            Some(found)
        }
        _ => None,
    }
}
