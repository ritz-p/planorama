use super::address::static_address;
use super::references::qualify;
use crate::model::{ProviderConfiguration, ProviderIdentity, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/providers.rs"]
mod tests;

pub(super) fn enrich(configuration: &Value, nodes: &mut [TerraformEntity]) {
    let mut resources = BTreeMap::new();
    collect(
        &configuration["root_module"],
        "",
        &configuration["provider_config"],
        &mut resources,
    );
    for node in nodes {
        if let Some((provider, configuration)) = resources.get(&static_address(&node.address)) {
            node.provider_configuration = Some(configuration.clone());
            if !node.provider.is_explicit() {
                node.provider = provider.clone();
            }
        }
    }
}

fn collect(
    module: &Value,
    scope: &str,
    providers: &Value,
    found: &mut BTreeMap<String, (ProviderIdentity, ProviderConfiguration)>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let (Some(address), Some(key)) = (
            resource["address"].as_str(),
            resource["provider_config_key"].as_str(),
        ) {
            let provider = providers[key]["full_name"]
                .as_str()
                .map(ProviderIdentity::from_source)
                // An explicit but unresolved binding must not imply HashiCorp AWS.
                .unwrap_or_else(|| ProviderIdentity::from_source("unknown"));
            let configuration = ProviderConfiguration {
                key: key.into(),
                alias: providers[key]["alias"].as_str().map(str::to_owned),
            };
            found.insert(qualify(scope, address), (provider, configuration));
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            collect(
                &call["module"],
                &qualify(scope, &format!("module.{name}")),
                providers,
                found,
            );
        }
    }
}
