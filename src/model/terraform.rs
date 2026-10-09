//! Value-free Terraform facts. No architecture roles, components or edge kinds.
use super::{Action, ChangeMetadata, EntityMode, ProviderIdentity};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerraformEntity {
    pub address: String,
    pub deposed_key: Option<String>,
    pub previous_address: Option<String>,
    pub metadata: ChangeMetadata,
    pub resource_type: String,
    pub provider: ProviderIdentity,
    pub module: String,
    pub action: Action,
    pub mode: EntityMode,
}

/// Resolved source/consumer indices in this plan's entity list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TerraformReference {
    pub from: usize,
    pub to: usize,
}

impl From<(usize, usize)> for TerraformReference {
    fn from((from, to): (usize, usize)) -> Self {
        Self { from, to }
    }
}

impl TerraformReference {
    pub fn endpoints(&self) -> (usize, usize) {
        (self.from, self.to)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerraformPlan {
    pub nodes: Vec<TerraformEntity>,
    pub edges: Vec<TerraformReference>,
    pub checks: Vec<super::CheckResult>,
    pub status: super::PlanStatus,
    pub remote_references: Vec<super::cross_state::RemoteReference>,
    pub outputs: Vec<super::StateOutput>,
    pub attributes: Vec<super::AttributeReference>,
    pub graph_references: Vec<super::AttributeReference>,
    pub drift: Vec<super::DriftRecord>,
    pub relevant_attributes: Vec<super::RelevantAttribute>,
}

#[cfg(test)]
#[test]
fn lifecycle_facts_are_independent_and_value_free() {
    let input = r#"{"format_version":"1.2","resource_changes":[
        {"address":"custom_item.x","type":"custom_item","previous_address":"custom_item.old","change":{"actions":["no-op"],"before":{"secret":"TOP_SECRET"},"after":{"secret":"TOP_SECRET"},"importing":{"id":"TOP_SECRET"}}},
        {"address":"custom_item.x","type":"custom_item","deposed":"old","change":{"actions":["delete"]}}
    ]}"#;
    let plan: TerraformPlan = crate::plan::parse(input).unwrap();
    assert_eq!(plan, crate::plan::parse(input).unwrap());
    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(
        plan.nodes[0].previous_address.as_deref(),
        Some("custom_item.old")
    );
    assert!(plan.nodes[0].metadata.import.unwrap().has_id);
    assert_eq!(plan.nodes[1].deposed_key.as_deref(), Some("old"));
    assert!(!format!("{plan:?}").contains("TOP_SECRET"));
    let before = plan.clone();
    let graph = crate::semantic::transform(&plan);
    assert_eq!(plan, before);
    assert_eq!(graph.nodes[0].metadata, plan.nodes[0].metadata);
    assert_eq!(graph.nodes[1].action, Action::Delete);
}
