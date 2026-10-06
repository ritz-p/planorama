use super::address::module_of;
use crate::classification;
use crate::model::{Action, EntityMode, Node, ProviderIdentity};
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
    let resource_type = resource["type"].as_str().unwrap_or("resource");
    let provider = resource["provider_name"]
        .as_str()
        .map(ProviderIdentity::from_source)
        .unwrap_or_else(|| ProviderIdentity::inferred(resource_type));
    Node {
        address: address.into(),
        resource_type: resource_type.into(),
        module,
        action,
        mode,
        role: classification::classify(&provider, resource_type),
        provider,
    }
}
