use super::address::{contextualize, matches_instance, static_address};
use crate::model::{DiagnosticReason, Node, Resolution};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn references(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(refs) = object.get("references").and_then(Value::as_array) {
                // Terraform emits traversal prefixes alongside the full reference.
                // Keep the most specific traversals within this expression.
                let refs: Vec<_> = refs.iter().filter_map(Value::as_str).collect();
                found.extend(
                    refs.iter()
                        .filter(|&&reference| {
                            !refs.iter().any(|&other| {
                                other != reference
                                    && other.strip_prefix(reference).is_some_and(|suffix| {
                                        suffix.starts_with('.') || suffix.starts_with('[')
                                    })
                            })
                        })
                        .map(|&reference| reference.to_owned()),
                );
            }
            for (key, value) in object {
                if key != "constant_value" && key != "references" {
                    references(value, found);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                references(value, found);
            }
        }
        _ => {}
    }
}

pub(super) fn qualify(scope: &str, reference: &str) -> String {
    match scope {
        "" => reference.into(),
        _ => format!("{scope}.{reference}"),
    }
}

pub(super) fn collect_config(
    module: &Value,
    scope: &str,
    inherited: &BTreeSet<String>,
    symbols: &mut BTreeMap<String, BTreeSet<String>>,
) {
    if let Some(locals) = module["locals"].as_object() {
        for (name, expression) in locals {
            let mut refs = BTreeSet::new();
            references(expression, &mut refs);
            symbols.insert(
                qualify(scope, &format!("local.{name}")),
                refs.into_iter().map(|r| qualify(scope, &r)).collect(),
            );
        }
    }
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let mut refs = BTreeSet::new();
            references(&resource["expressions"], &mut refs);
            references(&resource["count_expression"], &mut refs);
            references(&resource["for_each_expression"], &mut refs);
            refs.extend(
                resource["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut qualified: BTreeSet<_> = refs.into_iter().map(|r| qualify(scope, &r)).collect();
            qualified.extend(inherited.iter().cloned());
            symbols.insert(qualify(scope, address), qualified);
        }
    }
    if let Some(outputs) = module["outputs"].as_object() {
        for (name, output) in outputs {
            let mut refs = BTreeSet::new();
            references(output, &mut refs);
            symbols.insert(
                qualify(scope, &format!("output.{name}")),
                refs.into_iter().map(|r| qualify(scope, &r)).collect(),
            );
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            let child = qualify(scope, &format!("module.{name}"));
            if let Some(inputs) = call["expressions"].as_object() {
                for (name, expression) in inputs {
                    let mut refs = BTreeSet::new();
                    references(expression, &mut refs);
                    symbols.insert(
                        format!("{child}.var.{name}"),
                        refs.into_iter().map(|r| qualify(scope, &r)).collect(),
                    );
                }
            }
            let mut call_refs = BTreeSet::new();
            references(&call["count_expression"], &mut call_refs);
            references(&call["for_each_expression"], &mut call_refs);
            call_refs.extend(
                call["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut dependencies = inherited.clone();
            dependencies.extend(call_refs.into_iter().map(|r| qualify(scope, &r)));
            collect_config(&call["module"], &child, &dependencies, symbols);
            if let Some(outputs) = call["module"]["outputs"].as_object() {
                for name in outputs.keys() {
                    symbols.insert(
                        format!("{child}.{name}"),
                        BTreeSet::from([format!("{child}.output.{name}")]),
                    );
                }
            }
        }
    }
}

pub(super) fn prefix_match(reference: &str, key: &str) -> bool {
    reference == key
        || reference
            .strip_prefix(key)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

pub(super) fn resolve(
    nodes: &[Node],
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<crate::model::Edge> {
    let instances = instance_map(nodes);
    let mut edges = BTreeSet::new();
    for (target, node) in nodes.iter().enumerate() {
        if let Some(refs) = symbols.get(&static_address(&node.address)) {
            let refs = refs
                .iter()
                .map(|r| contextualize(r, &node.address))
                .collect();
            let resolution = resolve_sources(&refs, &instances, symbols);
            for source in resolution.sources {
                if source != target {
                    edges.insert((source, target));
                }
            }
        }
    }
    edges.into_iter().map(crate::model::Edge::from).collect()
}

pub(super) fn instance_map(nodes: &[Node]) -> BTreeMap<String, Vec<usize>> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.address.clone(), vec![i]))
        .collect()
}

pub(super) fn resolve_sources(
    refs: &BTreeSet<String>,
    instances: &BTreeMap<String, Vec<usize>>,
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Resolution {
    let mut pending: Vec<_> = refs.iter().cloned().map(|r| (r, false)).collect();
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut complete = !pending.is_empty();
    let mut issues = BTreeSet::new();
    while let Some((reference, exiting)) = pending.pop() {
        if exiting {
            active.remove(&reference);
            continue;
        }
        if active.contains(&reference) {
            complete = false;
            issues.insert(DiagnosticReason::AliasCycle);
            continue;
        }
        if !visited.insert(reference.clone()) {
            continue;
        }
        active.insert(reference.clone());
        pending.push((reference.clone(), true));
        let normalized = static_address(&reference);
        if super::address::dynamic_selection(&reference) {
            issues.insert(DiagnosticReason::DynamicInstanceSelection);
        }
        let resource_key = instances
            .keys()
            .map(|key| static_address(key))
            .filter(|key| prefix_match(&normalized, key))
            .max_by_key(String::len);
        if let Some(key) = resource_key {
            let matching: Vec<_> = instances
                .iter()
                .filter(|(address, _)| {
                    static_address(address) == key && matches_instance(&reference, address)
                })
                .flat_map(|(_, indices)| indices.iter().copied())
                .collect();
            if matching.is_empty() {
                complete = false;
                issues.insert(DiagnosticReason::UnresolvedReference);
            } else if matching.len() > 1 {
                issues.insert(DiagnosticReason::MultipleMatchingInstances);
            }
            sources.extend(matching);
            continue;
        }
        match symbols
            .iter()
            .filter(|(key, _)| prefix_match(&normalized, key))
            .max_by_key(|(key, _)| key.len())
        {
            Some((_, aliases)) if !aliases.is_empty() => {
                pending.extend(
                    aliases
                        .iter()
                        .map(|alias| (contextualize(alias, &reference), false)),
                );
            }
            None if normalized.starts_with("module.") => {
                let matching: Vec<_> = instances
                    .iter()
                    .filter(|(address, _)| {
                        prefix_match(&static_address(address), &normalized)
                            && matches_instance(&reference, address)
                    })
                    .flat_map(|(_, indices)| indices.iter().copied())
                    .collect();
                if matching.is_empty() {
                    complete = false;
                    issues.insert(DiagnosticReason::UnresolvedReference);
                } else if matching.len() > 1 {
                    issues.insert(DiagnosticReason::MultipleMatchingInstances);
                }
                sources.extend(matching);
            }
            _ => {
                complete = false;
                issues.insert(DiagnosticReason::UnresolvedReference);
            }
        }
    }
    Resolution {
        sources: sources.into_iter().collect(),
        complete,
        issues,
    }
}
