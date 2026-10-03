use super::address::module_of;
use crate::model::{Action, EntityMode, Node, ResourceRole};
use serde_json::Value;

#[cfg(test)]
#[path = "../../tests/unit/plan/entity.rs"]
mod tests;

pub(super) fn parse(resource: &Value, address: &str, action: Action) -> Node {
    let module = module_of(address);
    let local_address = match module.as_str() {
        "root" => address,
        module => address
            .strip_prefix(module)
            .and_then(|s| s.strip_prefix('.'))
            .unwrap_or(address),
    };
    let mode = match resource["mode"].as_str() {
        Some("data") => EntityMode::Data,
        Some("managed") => EntityMode::Managed,
        _ => match local_address.starts_with("data.") {
            true => EntityMode::Data,
            false => EntityMode::Managed,
        },
    };
    Node {
        address: address.into(),
        resource_type: resource["type"].as_str().unwrap_or("resource").into(),
        module,
        action,
        mode,
        role: ResourceRole::Unknown,
    }
}
