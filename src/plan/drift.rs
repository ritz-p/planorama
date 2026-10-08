use crate::model::{Action, DriftRecord, Node};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/drift.rs"]
mod tests;

fn compatible(left: &Node, right: &Node) -> bool {
    left.resource_type == right.resource_type
        && left.mode == right.mode
        && (left.provider == right.provider || (left.provider.is_aws() && right.provider.is_aws()))
}

pub(super) fn collect(plan: &Value, nodes: &mut Vec<Node>) -> Result<Vec<DriftRecord>, String> {
    let mut changes = Vec::new();
    for change in plan["resource_drift"].as_array().into_iter().flatten() {
        let address = change["address"]
            .as_str()
            .ok_or("resource drift is missing address")?;
        changes.push(super::entity::parse(
            change,
            address,
            super::parse_action(&change["change"]["actions"]),
        ));
    }
    super::providers::enrich(&plan["configuration"], &mut changes);
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for change in changes {
        groups
            .entry((change.address.clone(), change.deposed_key.clone()))
            .or_default()
            .push(change);
    }
    let mut found = Vec::new();
    for ((address, deposed_key), changes) in groups {
        let current = nodes
            .iter()
            .position(|node| node.address == address && node.deposed_key == deposed_key);
        let target = match current {
            Some(index) => Some(index),
            None if changes.iter().all(|node| compatible(&changes[0], node)) => {
                // Drift-only entries have no planned apply action. Do not copy
                // drift actions, imports or move provenance into apply metadata.
                let mut node = changes
                    .iter()
                    .find(|node| node.provider.is_explicit())
                    .unwrap_or(&changes[0])
                    .clone();
                node.action = Action::Unchanged;
                node.previous_address = None;
                nodes.push(node);
                Some(nodes.len() - 1)
            }
            None => None,
        };
        for change in changes {
            let matched = target.filter(|&index| compatible(&nodes[index], &change));
            if let Some(index) = matched {
                nodes[index].metadata.drift.insert(change.action);
            }
            found.push(DriftRecord {
                address: address.clone(),
                deposed_key: deposed_key.clone(),
                previous_address: change.previous_address,
                action: change.action,
                matched: matched.is_some(),
            });
        }
    }
    nodes.sort_by(|a, b| (&a.address, &a.deposed_key).cmp(&(&b.address, &b.deposed_key)));
    found.sort();
    Ok(found)
}
