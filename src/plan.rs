mod address;
mod attributes;
mod entity;
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
    if !plan["resource_changes"].is_array() && !plan["planned_values"].is_object() {
        return Err("expected plan JSON containing resource_changes or planned_values".into());
    }
    let mut nodes = BTreeMap::new();
    collect_values(&plan["planned_values"]["root_module"], &mut nodes, false);
    for change in plan["resource_changes"].as_array().into_iter().flatten() {
        let address = change["address"]
            .as_str()
            .ok_or("resource change is missing address")?;
        nodes.insert(
            address.into(),
            entity::parse(change, address, parse_action(&change["change"]["actions"])),
        );
    }
    let mut symbols = BTreeMap::new();
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
        if symbols.contains_key(&static_address(&address)) {
            nodes.entry(address).or_insert(node);
        }
    }
    let nodes: Vec<_> = nodes.into_values().collect();
    let edges = resolve(&nodes, &symbols);
    let attributes = attributes::collect(&plan["configuration"]["root_module"], &nodes, &symbols);
    Ok(TerraformGraph {
        graph: Graph { nodes, edges },
        attributes,
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

fn collect_values(module: &Value, nodes: &mut BTreeMap<String, Node>, data_only: bool) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let node = entity::parse(resource, address, Action::Unchanged);
            if data_only && node.mode != EntityMode::Data {
                continue;
            }
            nodes.entry(address.into()).or_insert(node);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        collect_values(child, nodes, data_only);
    }
}
