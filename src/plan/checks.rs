use crate::model::{CheckInstance, CheckResult, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/checks.rs"]
mod tests;

pub(super) fn parse(value: &Value, nodes: &[TerraformEntity]) -> Vec<CheckResult> {
    let Some(checks) = value.as_array() else {
        return Vec::new();
    };
    if checks.is_empty() {
        return Vec::new();
    }
    let mut resources = HashMap::with_capacity(nodes.len());
    for node in nodes {
        resources
            .entry(node.address.as_str())
            .and_modify(|entry| *entry = None)
            .or_insert_with(|| node.deposed_key.is_none().then_some(node));
    }
    let mut results: Vec<_> = checks
        .iter()
        .filter(|result| result.is_object())
        .map(|result| {
            let mut instances: Vec<_> = result["instances"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|instance| instance.is_object())
                .map(|instance| CheckInstance {
                    status: status(&instance["status"]),
                    resource: resource(&result["address"], &instance["address"], &resources),
                })
                .collect();
            instances.sort();
            CheckResult {
                status: status(&result["status"]),
                instances,
            }
        })
        .collect();
    results.sort();
    results
}

fn status(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn resource(
    address: &Value,
    instance: &Value,
    resources: &HashMap<&str, Option<&TerraformEntity>>,
) -> Option<String> {
    if address["kind"].as_str()? != "resource" {
        return None;
    }
    let display = instance["to_display"].as_str()?;
    let node = resources.get(display).copied().flatten()?;
    let mode = match node.mode {
        EntityMode::Managed => "managed",
        EntityMode::Data => "data",
    };
    if address["mode"].as_str()? != mode
        || address["type"].as_str()? != node.resource_type
        || address["to_display"].as_str()? != super::address::static_address(display)
    {
        return None;
    }
    Some(node.address.clone())
}
