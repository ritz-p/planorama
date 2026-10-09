use crate::model::{MultiPlan, PlanInput};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../tests/unit/plan/states.rs"]
mod tests;

pub fn parse_inputs(inputs: Vec<PlanInput>) -> Result<MultiPlan, String> {
    let mut ordered = BTreeMap::new();
    for input in inputs {
        if ordered.contains_key(&input.state_id) {
            return Err(format!("duplicate state ID: {}", input.state_id.as_str()));
        }
        ordered.insert(input.state_id, input.plan_json);
    }
    if ordered.is_empty() {
        return Err("at least one state input is required".into());
    }
    let states = ordered
        .into_iter()
        .map(|(id, json)| {
            super::parse(&json)
                .map_err(|error| format!("state {:?}: {error}", id.as_str()))
                .map(|graph| (id, graph))
        })
        .collect::<Result<_, _>>()?;
    Ok(MultiPlan { states })
}
