use super::Graph;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerraformGraph {
    pub graph: Graph,
    pub attributes: Vec<AttributeReference>,
    /// Dependency metadata stays separate from semantic attribute provenance.
    pub graph_references: Vec<AttributeReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeReference {
    pub target: usize,
    pub attribute: String,
    pub sources: Vec<usize>,
    pub complete: bool,
    pub issues: std::collections::BTreeSet<super::DiagnosticReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureGraph(pub Graph);

impl Deref for TerraformGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

impl Deref for ArchitectureGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
