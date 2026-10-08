mod address;
mod attributes;
mod drift;
mod entity;
mod operations;
mod providers;
mod references;
#[cfg(test)]
#[path = "../tests/unit/plan.rs"]
mod tests;

use crate::model::{Action, EntityMode, Graph, Node, TerraformGraph};
use address::static_address;
use references::{collect_config, resolve};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn parse(json: &str) -> Result<TerraformGraph, String> {
    let plan: Value = serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("invalid JSON: {e}"))?;
    let version = plan["format_version"]
        .as_str()
        .ok_or("missing format_version; expected terraform show -json output")?;
    match version.split('.').next() {
        Some("1") => {}
        _ => return Err(format!("unsupported plan format_version: {version}")),
    }
    if !plan["resource_changes"].is_array()
        && !plan["planned_values"].is_object()
        && !plan["resource_drift"].is_array()
    {
        return Err("expected plan JSON containing resource_changes or planned_values".into());
    }
    let mut nodes = BTreeMap::new();
    collect_values(&plan["planned_values"]["root_module"], &mut nodes, false);
    let planned_providers: BTreeMap<_, _> = nodes
        .iter()
        .filter(|(_, node)| node.provider.is_explicit())
        .map(|(key, node)| (key.clone(), node.provider.clone()))
        .collect();
    for change in plan["resource_changes"].as_array().into_iter().flatten() {
        let address = change["address"]
            .as_str()
            .ok_or("resource change is missing address")?;
        let mut node = entity::parse(change, address, parse_action(&change["change"]["actions"]));
        node.metadata = operations::parse(&change["change"]);
        if !node.provider.is_explicit() {
            if let Some(provider) = planned_providers.get(&(address.to_owned(), None)) {
                node.provider = provider.clone();
            }
        }
        nodes.insert((address.to_owned(), node.deposed_key.clone()), node);
    }
    let mut symbols = BTreeMap::new();
    // Real plan JSON stores root inputs outside configuration.root_module.
    // Register names only: values are neither resource provenance nor diagnostics.
    if let Some(variables) = plan["variables"].as_object() {
        for name in variables.keys() {
            symbols.insert(format!("var.{name}"), BTreeSet::new());
        }
    }
    collect_config(
        &plan["configuration"]["root_module"],
        "",
        &BTreeSet::new(),
        &mut symbols,
    );
    let mut prior_data = BTreeMap::new();
    collect_values(
        &plan["prior_state"]["values"]["root_module"],
        &mut prior_data,
        true,
    );
    for (address, node) in prior_data {
        if symbols.contains_key(&static_address(&address.0)) {
            nodes.entry(address).or_insert(node);
        }
    }
    let mut nodes: Vec<_> = nodes.into_values().collect();
    providers::enrich(&plan["configuration"], &mut nodes);
    let drift = drift::collect(&plan, &mut nodes)?;
    let edges = resolve(&nodes, &symbols);
    let attributes = attributes::collect(
        &plan["configuration"]["root_module"],
        &nodes,
        &symbols,
        false,
    );
    let graph_references = attributes::collect(
        &plan["configuration"]["root_module"],
        &nodes,
        &symbols,
        true,
    );
    Ok(TerraformGraph {
        graph: Graph { nodes, edges },
        attributes,
        graph_references,
        drift,
    })
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

fn collect_values(
    module: &Value,
    nodes: &mut BTreeMap<(String, Option<String>), Node>,
    data_only: bool,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let node = entity::parse(resource, address, Action::Unchanged);
            if data_only && node.mode != EntityMode::Data {
                continue;
            }
            nodes.entry((address.into(), None)).or_insert(node);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        collect_values(child, nodes, data_only);
    }
}
