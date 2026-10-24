use crate::model::{EntityMode, TerraformPlan};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
pub struct ResourceInventoryEntry {
    pub state: Option<String>,
    pub selector: String,
    pub address: String,
    pub resource_type: String,
    pub module: String,
    pub mode: &'static str,
    pub current: usize,
    pub deposed: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct InputInventory {
    pub modules: Vec<crate::ModuleInventoryEntry>,
    pub resources: Vec<ResourceInventoryEntry>,
}

pub fn inspect_resources(json: &str) -> Result<Vec<ResourceInventoryEntry>, crate::Error> {
    let plan = crate::generation::parse_input(json)?;
    Ok(inventory(&plan))
}

pub fn inspect_input(json: &str) -> Result<InputInventory, crate::Error> {
    let plan = crate::generation::parse_input(json)?;
    Ok(InputInventory {
        modules: crate::module_inventory::inventory(&plan)
            .map_err(|_| crate::Error::UnsupportedInput)?,
        resources: inventory(&plan),
    })
}

fn inventory(plan: &TerraformPlan) -> Vec<ResourceInventoryEntry> {
    let mut entries = BTreeMap::new();
    for node in &plan.nodes {
        let entry = entries
            .entry(&node.address)
            .or_insert_with(|| ResourceInventoryEntry {
                state: None,
                selector: node.address.clone(),
                address: node.address.clone(),
                resource_type: node.resource_type.clone(),
                module: node.module.clone(),
                mode: match node.mode {
                    EntityMode::Managed => "managed",
                    EntityMode::Data => "data",
                },
                current: 0,
                deposed: 0,
            });
        if node.deposed_key.is_some() {
            entry.deposed += 1;
        } else {
            entry.current += 1;
        }
    }
    entries.into_values().collect()
}

#[cfg(test)]
#[path = "../tests/unit/resource_inventory.rs"]
mod tests;
