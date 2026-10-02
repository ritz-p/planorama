use super::address::static_address;
use crate::model::Node;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn references(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(refs) = object.get("references").and_then(Value::as_array) {
                found.extend(refs.iter().filter_map(Value::as_str).map(str::to_owned));
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

fn qualify(scope: &str, reference: &str) -> String {
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

fn prefix_match(reference: &str, key: &str) -> bool {
    reference == key
        || reference
            .strip_prefix(key)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

pub(super) fn resolve(
    nodes: &[Node],
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<(usize, usize)> {
    let mut instances: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, node) in nodes.iter().enumerate() {
        instances
            .entry(static_address(&node.address))
            .or_default()
            .push(i);
    }
    let mut edges = BTreeSet::new();
    for (target, node) in nodes.iter().enumerate() {
        let key = static_address(&node.address);
        let mut pending: Vec<_> = symbols.get(&key).into_iter().flatten().cloned().collect();
        let mut visited = BTreeSet::new();
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
                Some((_, sources)) => {
                    for &source in sources {
                        if source != target {
                            edges.insert((source, target));
                        }
                    }
                }
                None => match symbols
                    .iter()
                    .filter(|(key, _)| prefix_match(&reference, key))
                    .max_by_key(|(key, _)| key.len())
                {
                    Some((_, aliases)) => pending.extend(aliases.iter().cloned()),
                    None if reference.starts_with("module.") => {
                        for (key, sources) in &instances {
                            if prefix_match(key, &reference) {
                                for &source in sources {
                                    if source != target {
                                        edges.insert((source, target));
                                    }
                                }
                            }
                        }
                    }
                    None => {}
                },
            }
        }
    }
    edges.into_iter().collect()
}
