use super::address::static_address;
use super::references::qualify;
use crate::model::{ProviderConfiguration, ProviderIdentity, RegionScope, TerraformEntity};
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
    for node in nodes.iter_mut().filter(|node| node.deposed_key.is_none()) {
        if let Some((provider, configuration, has_region_override)) =
            resources.get(&static_address(&node.address))
        {
            node.has_region_override = *has_region_override;
            if !node.provider.is_explicit() {
                node.provider = provider.clone();
            }
            let mut configuration = configuration.clone();
            if !node.provider.is_aws() {
                configuration.region = None;
                configuration.account = None;
                configuration.account_conflict = false;
            }
            node.provider_configuration = Some(configuration);
        }
    }
}

fn collect(
    module: &Value,
    scope: &str,
    providers: &Value,
    found: &mut BTreeMap<String, (ProviderIdentity, ProviderConfiguration, bool)>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let (Some(address), Some(key)) = (
            resource["address"].as_str(),
            resource["provider_config_key"].as_str(),
        ) {
            let provider = providers[key]["full_name"]
                .as_str()
                .map(ProviderIdentity::from_source)
                .unwrap_or_else(|| ProviderIdentity::from_source("unknown"));
            let configuration = ProviderConfiguration {
                key: key.into(),
                alias: providers[key]["alias"].as_str().map(str::to_owned),
                region: static_region(&provider, &providers[key]["expressions"]["region"]),
                account: None,
                account_conflict: false,
            };
            found.insert(
                qualify(scope, address),
                (
                    provider,
                    configuration,
                    resource["expressions"].get("region").is_some(),
                ),
            );
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

pub(super) fn static_account(
    provider: &ProviderIdentity,
    expressions: &Value,
) -> (Option<crate::model::AwsAccountScope>, bool) {
    if !provider.is_aws() {
        return (None, false);
    }
    let mut accounts = std::collections::BTreeSet::new();
    let mut incomplete = false;
    for name in ["assume_role", "assume_role_with_web_identity"] {
        let Some(value) = expressions.get(name) else {
            continue;
        };
        let Some(blocks) = value.as_array() else {
            return (None, false);
        };
        for block in blocks {
            let expression = &block["role_arn"];
            let account = expression
                .get("references")
                .is_none()
                .then(|| expression["constant_value"].as_str().and_then(role_account))
                .flatten();
            if let Some(account) = account {
                accounts.insert(account);
            } else {
                incomplete = true;
            }
        }
    }
    let conflict = accounts.len() > 1;
    if incomplete || conflict {
        (None, conflict)
    } else {
        (accounts.pop_first(), false)
    }
}

fn role_account(arn: &str) -> Option<crate::model::AwsAccountScope> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.len() != 6
        || parts[0] != "arn"
        || parts[1] != "aws"
        || parts[2] != "iam"
        || !parts[3].is_empty()
    {
        return None;
    }
    let role = parts[5].strip_prefix("role/")?;
    let name = role.rsplit('/').next()?;
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_+=,.@-".contains(&b))
        || !role.bytes().all(|b| (0x21..=0x7e).contains(&b))
    {
        return None;
    }
    crate::model::AwsAccountScope::new(parts[4])
}

fn static_region(provider: &ProviderIdentity, expression: &Value) -> Option<RegionScope> {
    if !provider.is_aws() || expression.get("references").is_some() {
        return None;
    }
    expression["constant_value"]
        .as_str()
        .filter(|region| {
            !region.is_empty()
                && region
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        .map(|region| RegionScope(region.to_owned()))
}
