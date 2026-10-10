//! Domain contracts shared by ingestion, architecture, view and presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Unchanged,
    Create,
    Update,
    Delete,
    Replace,
    Read,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub entity: ArchitectureEntity,
    pub address: String,
    pub deposed_key: Option<String>,
    pub previous_address: Option<String>,
    pub metadata: ChangeMetadata,
    pub resource_type: String,
    pub provider: ProviderIdentity,
    pub provider_configuration: Option<ProviderConfiguration>,
    pub module: String,
    pub action: Action,
    pub mode: EntityMode,
    pub role: ResourceRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityMode {
    Managed,
    Data,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceRole {
    Container,
    Node,
    Connector,
    Association,
    Policy,
    Controller,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    pub relationships: Vec<ArchitectureRelationship>,
    pub components: Vec<SyntheticComponent>,
    pub checks: Vec<CheckResult>,
    pub status: PlanStatus,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Status reported by one input plan; absent flags remain unknown.
/// Multiple plans must retain their own status rather than combining flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanStatus {
    pub applyable: Option<bool>,
    pub complete: Option<bool>,
    pub errored: Option<bool>,
}

mod address;
pub(crate) mod architecture;
pub use architecture::{ArchitectureEntity, ArchitectureId, TerraformEntityId};
mod checks;
mod components;
pub(crate) use address::module_of;
pub use checks::{CheckInstance, CheckResult};
pub use components::SyntheticComponent;
pub mod cross_state;
mod diagnostics;
mod graphs;
mod relationships;
pub use relationships::{ArchitectureRelationship, RelationshipProvenance};
mod operations;
mod provider;
mod states;
mod terraform;
pub(crate) use diagnostics::Resolution;
pub use diagnostics::{Diagnostic, DiagnosticReason};
pub use graphs::{ArchitectureGraph, AttributeReference, StateOutput};
pub use operations::{
    AttributePathStep, ChangeMetadata, DriftRecord, ImportMetadata, RelevantAttribute, StateRemoval,
};
pub use provider::{ProviderConfiguration, ProviderIdentity};
pub use states::{MultiPlan, PlanInput, StateId};
pub use terraform::{TerraformEntity, TerraformPlan, TerraformReference};

#[allow(
    dead_code,
    reason = "Semantic transformations will introduce non-dependency edge kinds"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    Dependency,
    Association,
    Connection,
    Containment,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
    pub change: Option<EdgeChange>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeChange {
    pub address: String,
    pub previous_address: Option<String>,
    pub metadata: ChangeMetadata,
    pub action: Action,
}

impl Edge {
    pub fn endpoints(&self) -> (usize, usize) {
        (self.from, self.to)
    }
}

impl From<(usize, usize)> for Edge {
    fn from((from, to): (usize, usize)) -> Self {
        Self {
            from,
            to,
            kind: EdgeKind::Dependency,
            change: None,
        }
    }
}
