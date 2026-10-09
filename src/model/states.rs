use super::TerraformGraph;
use std::collections::BTreeMap;

/// Explicit caller-supplied identity, never inferred from a path or backend.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateId(String);

impl StateId {
    pub fn new(value: &str) -> Result<Self, String> {
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err("state ID must contain 1-64 ASCII letters, digits, '.', '_' or '-'".into());
        }
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct PlanInput {
    pub state_id: StateId,
    pub plan_json: String,
}

/// Graph indices, Terraform addresses, deposed identities, checks, and logical
/// component IDs are local to each state partition. Cross-state identity is
/// (StateId, local identity); no flattening or cross-state inference occurs.
#[derive(Debug, PartialEq, Eq)]
pub struct MultiPlan {
    pub states: BTreeMap<StateId, TerraformGraph>,
}
