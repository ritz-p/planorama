use super::Graph;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateOutput {
    pub name: String,
    pub sources: Vec<String>,
    pub complete: bool,
    pub issues: std::collections::BTreeSet<super::DiagnosticReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeReference {
    pub target: usize,
    pub attribute: String,
    pub sources: Vec<usize>,
    pub complete: bool,
    pub collection_ids_complete: bool,
    pub scalar_id_matches: bool,
    pub ids_only: bool,
    pub issues: std::collections::BTreeSet<super::DiagnosticReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchitectureGraph(pub Graph);

impl Deref for ArchitectureGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
