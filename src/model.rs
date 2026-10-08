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
    pub address: String,
    pub deposed_key: Option<String>,
    pub previous_address: Option<String>,
    pub metadata: ChangeMetadata,
    pub resource_type: String,
    pub provider: ProviderIdentity,
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
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

mod address;
pub(crate) use address::module_of;
mod diagnostics;
mod graphs;
mod operations;
mod provider;
pub(crate) use diagnostics::Resolution;
pub use diagnostics::{Diagnostic, DiagnosticReason};
pub use graphs::{ArchitectureGraph, AttributeReference, TerraformGraph};
pub use operations::{ChangeMetadata, DriftRecord, ImportMetadata, StateRemoval};
pub use provider::ProviderIdentity;

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
