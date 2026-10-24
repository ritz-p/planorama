use crate::model::{TerraformPlan, modules::Address};

#[derive(Debug, serde::Serialize)]
pub struct ModuleInventoryEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    pub selector: Option<String>,
    pub instance: bool,
    pub direct: usize,
    pub subtree: usize,
}

pub fn inspect_modules(json: &str) -> Result<Vec<ModuleInventoryEntry>, crate::Error> {
    let plan = crate::generation::parse_input(json)?;
    inventory(&plan).map_err(|_| crate::Error::UnsupportedInput)
}

pub(crate) fn inventory(plan: &TerraformPlan) -> Result<Vec<ModuleInventoryEntry>, String> {
    let resources = plan
        .nodes
        .iter()
        .map(|node| Address::resource(&node.address))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(plan
        .modules
        .0
        .iter()
        .map(|(module, actual)| {
            let subtree = resources
                .iter()
                .filter(|resource| module.contains(resource))
                .count();
            let direct = resources
                .iter()
                .filter(|resource| module.depth() == resource.depth() && module.contains(resource))
                .count();
            ModuleInventoryEntry {
                state: None,
                selector: (module.depth() > 0).then(|| module.to_string()),
                instance: *actual,
                direct,
                subtree,
            }
        })
        .collect())
}

#[cfg(test)]
#[path = "../tests/unit/module_inventory.rs"]
mod tests;
