use super::Graph;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerraformGraph {
    pub graph: Graph,
    pub attributes: Vec<AttributeReference>,
    /// Dependency metadata stays separate from semantic attribute provenance.
    pub graph_references: Vec<AttributeReference>,
    pub drift: Vec<super::DriftRecord>,
    pub relevant_attributes: Vec<super::RelevantAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeReference {
    pub target: usize,
    pub attribute: String,
    pub sources: Vec<usize>,
    pub complete: bool,
    /// Value-free proof that the entire planned ID collection matches its sources.
    pub collection_ids_complete: bool,
    /// Known scalar planned value equals the single referenced resource ID.
    pub scalar_id_matches: bool,
    /// All resolved resource traversals explicitly select `.id`.
    pub ids_only: bool,
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
