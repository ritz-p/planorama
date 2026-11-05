use super::address::static_address;
use super::references::qualify;
use crate::model::{
    Action, EntityMode, ProviderConfiguration, ProviderIdentity, RegionScope, TerraformEntity,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/providers.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/unit/plan/region_overrides.rs"]
mod region_override_tests;

const REGION_OVERRIDE_CORE_VERSIONS: &[&str] = &["1.9.8", "1.16.6"];

struct ResourceConfiguration<'a> {
    provider: ProviderIdentity,
    configuration: ProviderConfiguration,
    resource: &'a Value,
    verified_version: bool,
    ambiguous: bool,
}

pub(super) fn enrich(plan: &Value, nodes: &mut [TerraformEntity]) {
    let configuration = &plan["configuration"];
    let mut current_changes = BTreeMap::new();
    for change in plan["resource_changes"].as_array().into_iter().flatten() {
        if change.get("deposed").is_none() {
            if let Some(address) = change["address"].as_str() {
                current_changes
                    .entry(address)
                    .and_modify(|entry| *entry = None)
                    .or_insert(Some(change));
            }
        }
    }
    let mut resources = BTreeMap::new();
    collect(
        &configuration["root_module"],
        "",
        &configuration["provider_config"],
        plan["terraform_version"].as_str(),
        &mut resources,
    );
    for node in nodes.iter_mut().filter(|node| node.deposed_key.is_none()) {
        if let Some(binding) = resources.get(&static_address(&node.address)) {
            let ResourceConfiguration {
                provider,
                configuration,
                resource,
                verified_version,
                ambiguous,
            } = binding;
            let expression = resource["expressions"].get("region");
            node.has_region_override = expression.is_some() || *ambiguous;
            if !node.provider.is_explicit() {
                node.provider = provider.clone();
            }
            if *verified_version
                && !ambiguous
                && node.provider == *provider
                && resource["type"] == node.resource_type
                && resource["mode"] == "managed"
            {
                if let Some(expression) = expression {
                    node.region_override = current_changes
                        .get(node.address.as_str())
                        .copied()
                        .flatten()
                        .and_then(|change| {
                            resource_region(node, expression, &plan["terraform_version"], change)
                        });
                }
            }
            if *verified_version && node.action == Action::Delete {
                node.has_region_override = true;
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

fn collect<'a>(
    module: &'a Value,
    scope: &str,
    providers: &Value,
    core: Option<&str>,
    found: &mut BTreeMap<String, ResourceConfiguration<'a>>,
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
            let address = qualify(scope, address);
            if let Some(existing) = found.get_mut(&address) {
                existing.ambiguous = true;
                continue;
            }
            found.insert(
                address,
                ResourceConfiguration {
                    provider,
                    configuration,
                    resource,
                    ambiguous: false,
                    verified_version: match providers[key]["version_constraint"].as_str() {
                        Some("6.0.0" | "= 6.0.0") => true,
                        Some("6.70.0" | "= 6.70.0") => core == Some("1.9.8"),
                        _ => false,
                    },
                },
            );
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            collect(
                &call["module"],
                &qualify(scope, &format!("module.{name}")),
                providers,
                core,
                found,
            );
        }
    }
}

fn resource_region(
    node: &TerraformEntity,
    expression: &Value,
    terraform_version: &Value,
    change: &Value,
) -> Option<RegionScope> {
    if node.mode != EntityMode::Managed
        || !matches!(
            node.action,
            Action::Create | Action::Update | Action::Unchanged | Action::Replace
        )
        || !matches!(
            node.resource_type.as_str(),
            "aws_vpc" | "aws_subnet" | "aws_cloudwatch_log_group"
        )
        || !terraform_version
            .as_str()
            .is_some_and(|version| REGION_OVERRIDE_CORE_VERSIONS.contains(&version))
    {
        return None;
    }
    let region = static_region(&node.provider, expression)?;
    if !matches!(
        region.0.as_str(),
        "us-east-1"
            | "us-east-2"
            | "us-west-1"
            | "us-west-2"
            | "eu-west-1"
            | "eu-west-2"
            | "eu-west-3"
            | "eu-central-1"
            | "ap-northeast-1"
            | "ap-northeast-2"
            | "ap-southeast-1"
            | "ap-southeast-2"
    ) {
        return None;
    }
    if change["change"]["after"]["region"] != region.0
        || !matches!(
            change["change"]["after_unknown"].get("region"),
            None | Some(Value::Bool(false))
        )
        || change["change"]["after_unknown"] == true
        || change["change"]["after_sensitive"] == true
        || change["change"]["after_sensitive"]["region"] == true
    {
        return None;
    }
    Some(region)
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
