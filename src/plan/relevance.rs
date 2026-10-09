use crate::model::{AttributePathStep, DriftRecord, Node, RelevantAttribute};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/relevance.rs"]
mod tests;

pub(super) fn collect(
    plan: &Value,
    nodes: &mut [Node],
    drift: &mut [DriftRecord],
) -> Vec<RelevantAttribute> {
    let mut addresses = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        addresses
            .entry(node.address.clone())
            .and_modify(|entry| *entry = None)
            .or_insert_with(|| node.deposed_key.is_none().then_some(index));
    }
    let mut sensitivity: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for field in ["resource_changes", "resource_drift"] {
        for change in plan[field].as_array().into_iter().flatten() {
            if let Some(address) = change["address"].as_str() {
                sensitivity.entry(address).or_default().extend([
                    &change["change"]["before_sensitive"],
                    &change["change"]["after_sensitive"],
                ]);
            }
        }
    }
    collect_sensitivity(&plan["planned_values"]["root_module"], &mut sensitivity);
    collect_sensitivity(
        &plan["prior_state"]["values"]["root_module"],
        &mut sensitivity,
    );
    let mut found = Vec::new();
    for relevant in plan["relevant_attributes"].as_array().into_iter().flatten() {
        let (Some(resource), Some(steps)) = (
            relevant["resource"].as_str(),
            relevant["attribute"].as_array(),
        ) else {
            continue;
        };
        if !steps
            .iter()
            .all(|step| step.is_string() || step.as_u64().is_some())
        {
            continue;
        }
        let index = addresses.get(resource).copied().flatten();
        // Unresolved sources provide no trustworthy sensitivity context.
        let hidden = index.is_none()
            || sensitivity.get(resource).is_some_and(|trees| {
                trees
                    .iter()
                    .any(|tree| super::operations::sensitive_path(tree, steps))
            });
        let path = (!hidden).then(|| {
            steps
                .iter()
                .map(|step| match step.as_str() {
                    Some(name) => AttributePathStep::Attribute(name.into()),
                    None => AttributePathStep::Index(step.as_u64().expect("validated path step")),
                })
                .collect::<Vec<_>>()
        });
        if let Some(index) = index {
            let paths = nodes[index]
                .metadata
                .relevant_attributes
                .get_or_insert_with(Vec::new);
            if let Some(path) = &path {
                paths.push(path.clone());
            }
        }
        found.push(RelevantAttribute {
            resource: resource.into(),
            path,
            matched: index.is_some(),
        });
    }
    for node in nodes.iter_mut() {
        if let Some(paths) = &mut node.metadata.relevant_attributes {
            paths.sort();
            paths.dedup();
        }
    }
    for record in drift {
        record.relevant = record.matched
            && record.deposed_key.is_none()
            && addresses
                .get(&record.address)
                .copied()
                .flatten()
                .is_some_and(|index| nodes[index].metadata.relevant_attributes.is_some());
    }
    found.sort();
    found.dedup();
    found
}

fn collect_sensitivity<'a>(module: &'a Value, found: &mut BTreeMap<&'a str, Vec<&'a Value>>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found
                .entry(address)
                .or_default()
                .push(&resource["sensitive_values"]);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        collect_sensitivity(child, found);
    }
}
