use crate::model::{Node, cross_state::RemoteReference};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn collect(module: &Value, nodes: &[Node]) -> Vec<RemoteReference> {
    let mut expressions = BTreeMap::new();
    walk(module, "", &mut expressions);
    let mut result = BTreeSet::new();
    for node in nodes.iter().filter(|n| n.deposed_key.is_none()) {
        if let Some(refs) = expressions.get(&super::address::static_address(&node.address)) {
            for reference in refs {
                let reference = super::address::contextualize(reference, &node.address);
                let parts = super::address::parts(&reference);
                let Some(i) = parts
                    .windows(2)
                    .position(|p| p == ["data", "terraform_remote_state"])
                else {
                    continue;
                };
                if parts.len() != i + 5 || parts[i + 3] != "outputs" {
                    continue;
                }
                let remote = parts[..i + 3].join(".");
                if super::address::dynamic_selection(
                    &remote,
                    Some(&super::address::static_address(&remote)),
                ) || parts[i + 4].contains(['[', ']'])
                {
                    continue;
                }
                result.insert(RemoteReference {
                    consumer: node.address.clone(),
                    remote,
                    output: parts[i + 4].into(),
                });
            }
        }
    }
    result.into_iter().collect()
}

fn walk(module: &Value, scope: &str, found: &mut BTreeMap<String, BTreeSet<String>>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let mut refs = BTreeSet::new();
            super::references::references(&resource["expressions"], &mut refs);
            found.insert(
                super::references::qualify(scope, address),
                refs.into_iter()
                    .map(|r| super::references::qualify(scope, &r))
                    .collect(),
            );
        }
    }
    for (name, call) in module["module_calls"].as_object().into_iter().flatten() {
        walk(
            &call["module"],
            &super::references::qualify(scope, &format!("module.{name}")),
            found,
        );
    }
}
