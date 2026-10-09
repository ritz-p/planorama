use crate::model::{CheckInstance, CheckResult, EntityMode, Node};
use serde_json::Value;

#[cfg(test)]
#[path = "../../tests/unit/plan/checks.rs"]
mod tests;

pub(super) fn parse(value: &Value, nodes: &[Node]) -> Vec<CheckResult> {
    let mut results: Vec<_> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter(|result| result.is_object())
        .map(|result| {
            let mut instances: Vec<_> = result["instances"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|instance| instance.is_object())
                .map(|instance| CheckInstance {
                    status: status(&instance["status"]),
                    resource: resource(&result["address"], &instance["address"], nodes),
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

fn resource(address: &Value, instance: &Value, nodes: &[Node]) -> Option<String> {
    if address["kind"].as_str()? != "resource" {
        return None;
    }
    let display = instance["to_display"].as_str()?;
    let mut matches = nodes.iter().filter(|node| node.address == display);
    let node = matches.next()?;
    // A check address cannot select between current and deposed objects.
    if matches.next().is_some() || node.deposed_key.is_some() {
        return None;
    }
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
