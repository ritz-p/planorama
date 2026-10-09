use super::Graph;
use std::ops::Deref;

/// Root outputs are state-local; sources are Terraform-native addresses.
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

impl Deref for ArchitectureGraph {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
