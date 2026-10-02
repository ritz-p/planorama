//! Convert Terraform plan JSON into a renderer-independent graph.

mod address;
mod references;
#[cfg(test)]
mod tests;

use crate::model::{Action, Graph, Node};
use address::{module_of, static_address};
use references::{collect_config, resolve};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn parse(json: &str) -> Result<Graph, String> {
    let plan: Value = serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("invalid JSON: {e}"))?;
    let version = plan["format_version"]
        .as_str()
        .ok_or("missing format_version; expected terraform show -json output")?;
    match version.split('.').next() {
        Some("1") => {}
        _ => return Err(format!("unsupported plan format_version: {version}")),
    }
    if !plan["resource_changes"].is_array() && !plan["planned_values"].is_object() {
        return Err("expected plan JSON containing resource_changes or planned_values".into());
    }
    let mut nodes = BTreeMap::new();
    collect_values(&plan["planned_values"]["root_module"], &mut nodes, false);
    // Include deleted resources absent from planned_values.
    for change in plan["resource_changes"].as_array().into_iter().flatten() {
        let address = change["address"]
            .as_str()
            .ok_or("resource change is missing address")?;
        nodes.insert(
            address.into(),
            Node {
                address: address.into(),
                module: module_of(address),
                resource_type: change["type"].as_str().unwrap_or("resource").into(),
                action: parse_action(&change["change"]["actions"]),
            },
        );
    }
    let mut symbols = BTreeMap::new();
    collect_config(
        &plan["configuration"]["root_module"],
        "",
        &BTreeSet::new(),
        &mut symbols,
    );
    // Terraform can omit data sources already read during plan from both
    // planned_values and resource_changes. Recover only data sources still
    // in configuration, without reintroducing removed or managed resources.
    let mut prior_data = BTreeMap::new();
    collect_values(
        &plan["prior_state"]["values"]["root_module"],
        &mut prior_data,
        true,
    );
    for (address, node) in prior_data {
        if symbols.contains_key(&static_address(&address)) {
            nodes.entry(address).or_insert(node);
        }
    }
    let nodes: Vec<_> = nodes.into_values().collect();
    let edges = resolve(&nodes, &symbols);
    Ok(Graph { nodes, edges })
}

fn parse_action(actions: &Value) -> Action {
    let actions: Vec<_> = actions
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    match actions.as_slice() {
        actions if actions.contains(&"create") && actions.contains(&"delete") => Action::Replace,
        ["no-op"] => Action::Unchanged,
        ["create"] => Action::Create,
        ["update"] => Action::Update,
        ["delete"] => Action::Delete,
        ["read"] => Action::Read,
        _ => Action::Other,
    }
}

fn collect_values(module: &Value, nodes: &mut BTreeMap<String, Node>, data_only: bool) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if data_only && resource["mode"] != "data" {
            continue;
        }
        if let Some(address) = resource["address"].as_str() {
            nodes.entry(address.into()).or_insert_with(|| Node {
                address: address.into(),
                module: module_of(address),
                resource_type: resource["type"].as_str().unwrap_or("resource").into(),
                action: Action::Unchanged,
            });
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        collect_values(child, nodes, data_only);
    }
}
