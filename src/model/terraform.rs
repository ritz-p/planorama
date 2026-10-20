use super::{Action, ChangeMetadata, EntityMode, ProviderConfiguration, ProviderIdentity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WafScope {
    Regional,
    CloudFront,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerraformEntity {
    pub waf_scope: Option<WafScope>,
    pub route_entries: Vec<super::RouteEntry>,
    pub address: String,
    pub deposed_key: Option<String>,
    pub previous_address: Option<String>,
    pub metadata: ChangeMetadata,
    pub resource_type: String,
    pub provider: ProviderIdentity,
    pub provider_configuration: Option<ProviderConfiguration>,
    pub has_region_override: bool,
    pub region_override: Option<super::RegionScope>,
    pub module: String,
    pub action: Action,
    pub mode: EntityMode,
}

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
    pub modules: super::modules::Inventory,
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
#[path = "../../tests/unit/model/terraform.rs"]
mod tests;
