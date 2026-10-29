use crate::model::{Action, RouteEntry, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

fn configurations<'a>(module: &'a Value, scope: &str, found: &mut BTreeMap<String, &'a Value>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found.insert(
                super::references::qualify(scope, address),
                &resource["expressions"],
            );
        }
    }
    for (name, call) in module["module_calls"].as_object().into_iter().flatten() {
        configurations(
            &call["module"],
            &super::references::qualify(scope, &format!("module.{name}")),
            found,
        );
    }
}

fn marked(value: &Value) -> bool {
    match value {
        Value::Bool(true) => true,
        Value::Array(values) => values.iter().any(marked),
        Value::Object(values) => values.values().any(marked),
        _ => false,
    }
}

fn sensitive_resources(module: &Value, found: &mut std::collections::BTreeSet<String>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if marked(&resource["sensitive_values"]) {
            if let Some(address) = resource["address"].as_str() {
                found.insert(address.into());
            }
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        sensitive_resources(child, found);
    }
}

fn destination(values: &Value, expressions: bool) -> Option<String> {
    let mut destinations = Vec::new();
    for (field, kind) in [
        ("destination_cidr_block", "IPv4"),
        ("cidr_block", "IPv4"),
        ("destination_ipv6_cidr_block", "IPv6"),
        ("ipv6_cidr_block", "IPv6"),
        ("destination_prefix_list_id", "Prefix list"),
    ] {
        if expressions && !values[field]["references"].is_null() {
            return None;
        }
        let value = if expressions {
            &values[field]["constant_value"]
        } else {
            &values[field]
        };
        let Some(value) = value.as_str().filter(|v| !v.is_empty()) else {
            continue;
        };
        let valid = match kind {
            "Prefix list" => value
                .strip_prefix("pl-")
                .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit())),
            _ => value.split_once('/').is_some_and(|(address, prefix)| {
                let Ok(prefix) = prefix.parse::<u8>() else {
                    return false;
                };
                match kind {
                    "IPv4" => address.parse::<std::net::Ipv4Addr>().is_ok() && prefix <= 32,
                    _ => address.parse::<std::net::Ipv6Addr>().is_ok() && prefix <= 128,
                }
            }),
        };
        if !valid {
            return None;
        }
        destinations.push(format!("{kind}: {value}"));
    }
    match destinations.as_slice() {
        [destination] => Some(destination.clone()),
        _ => None,
    }
}

fn planned_values<'a>(module: &'a Value, found: &mut BTreeMap<&'a str, &'a Value>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found.insert(address, &resource["values"]);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        planned_values(child, found);
    }
}

fn known_ids<'a>(
    plan: &'a Value,
    nodes: &[TerraformEntity],
) -> BTreeMap<&'a str, Option<crate::model::TerraformEntityId>> {
    let mut values = BTreeMap::new();
    planned_values(&plan["planned_values"]["root_module"], &mut values);
    let mut ids = BTreeMap::new();
    let changes: BTreeMap<_, _> = plan["resource_changes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["deposed"].is_null())
        .filter_map(|c| c["address"].as_str().map(|a| (a, c)))
        .collect();
    for node in nodes.iter().filter(|n| n.deposed_key.is_none()) {
        let change = changes.get(node.address.as_str());
        let (value, unknown) = match change {
            Some(change) => (
                &change["change"]["after"],
                &change["change"]["after_unknown"],
            ),
            None => (
                values
                    .get(node.address.as_str())
                    .copied()
                    .unwrap_or(&Value::Null),
                &Value::Null,
            ),
        };
        if unknown == &Value::Bool(true) || marked(&unknown["id"]) {
            continue;
        }
        if let Some(id) = value["id"].as_str().filter(|s| !s.is_empty()) {
            ids.entry(id)
                .and_modify(|entry| *entry = None)
                .or_insert_with(|| Some(node.into()));
        }
    }
    ids
}

fn inline_entry(
    entry: &Value,
    expressions: bool,
    safe: bool,
    ids: &BTreeMap<&str, Option<crate::model::TerraformEntityId>>,
) -> RouteEntry {
    if !safe {
        return RouteEntry::default();
    }
    let candidates: Vec<_> = entry
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, value)| {
            (name.ends_with("_id") || name.ends_with("_arn"))
                && name.as_str() != "destination_prefix_list_id"
                && !value.is_null()
                && value.as_str() != Some("")
        })
        .collect();
    let (target_attribute, target) = match candidates.as_slice() {
        [(attribute, value)] => {
            let value = if expressions {
                if !value["references"].is_null() {
                    &Value::Null
                } else {
                    &value["constant_value"]
                }
            } else {
                value
            };
            (
                Some((*attribute).clone()),
                value.as_str().and_then(|id| ids.get(id)).cloned().flatten(),
            )
        }
        _ => (None, None),
    };
    RouteEntry {
        destination: destination(entry, expressions),
        target,
        target_attribute,
        ..Default::default()
    }
}

pub(super) fn enrich(plan: &Value, nodes: &mut [TerraformEntity]) {
    let ids = known_ids(plan, nodes);
    let mut configs = BTreeMap::new();
    let mut planned = BTreeMap::new();
    planned_values(&plan["planned_values"]["root_module"], &mut planned);
    let mut sensitive = std::collections::BTreeSet::new();
    sensitive_resources(&plan["planned_values"]["root_module"], &mut sensitive);
    sensitive_resources(
        &plan["prior_state"]["values"]["root_module"],
        &mut sensitive,
    );
    configurations(&plan["configuration"]["root_module"], "", &mut configs);
    for node in nodes {
        if !node.provider.is_aws()
            || node.mode != crate::model::EntityMode::Managed
            || !matches!(node.resource_type.as_str(), "aws_route" | "aws_route_table")
        {
            continue;
        }
        let expressions = configs
            .get(&super::address::static_address(&node.address))
            .copied()
            .unwrap_or(&Value::Null);
        let change = plan["resource_changes"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| {
                c["address"].as_str() == Some(&node.address)
                    && c["deposed"].as_str() == node.deposed_key.as_deref()
            })
            .map(|c| &c["change"])
            .unwrap_or(&Value::Null);
        let safe = node.deposed_key.is_none()
            && node.action != Action::Delete
            && !sensitive.contains(&node.address)
            && !marked(&change["after_sensitive"])
            && !marked(&change["before_sensitive"]);
        let destination_known = change["after_unknown"] != Value::Bool(true)
            && [
                "destination_cidr_block",
                "destination_ipv6_cidr_block",
                "destination_prefix_list_id",
                "route",
            ]
            .iter()
            .all(|field| !marked(&change["after_unknown"][field]));
        let safe = safe && destination_known;
        if node.resource_type == "aws_route" {
            node.route_entries = vec![RouteEntry {
                destination: safe.then(|| destination(expressions, true)).flatten(),
                ..Default::default()
            }];
        } else if let Some(entries) = expressions["route"]
            .as_array()
            .or_else(|| expressions["route"]["constant_value"].as_array())
        {
            let expression_entries = expressions["route"].is_array();
            node.route_entries = entries
                .iter()
                .map(|entry| inline_entry(entry, expression_entries, safe, &ids))
                .collect();
            node.route_entries.sort();
        } else if !expressions["route"].is_null() || marked(&change["after_unknown"]["route"]) {
            node.route_entries = vec![RouteEntry::default()];
        } else {
            let values = if node.action == Action::Delete {
                &change["before"]
            } else {
                change
                    .get("after")
                    .or_else(|| planned.get(node.address.as_str()).copied())
                    .unwrap_or(&Value::Null)
            };
            if let Some(entries) = values["route"].as_array() {
                node.route_entries = vec![RouteEntry::default(); entries.len()];
            }
        }
    }
}
