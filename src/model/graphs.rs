use super::Graph;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateOutput {
    pub name: String,
    pub action: Option<super::Action>,
    pub sources: Vec<String>,
    pub complete: bool,
    pub issues: std::collections::BTreeSet<super::DiagnosticReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeReference {
    pub indexed_block_lengths: Vec<usize>,
    pub target: usize,
    pub attribute: String,
    pub has_references: bool,
    pub sources: Vec<usize>,
    pub complete: bool,
    pub instances_explicit: bool,
    pub collection_ids_complete: bool,
    pub scalar_id_matches: bool,
    pub scalar_name_matches: bool,
    pub arn_values_match: bool,
    pub messaging_endpoint_verified: bool,
    pub monitoring_verified: bool,
    pub source_attributes: std::collections::BTreeSet<String>,
    pub service_reference_verified: bool,
    pub service_reference_owner_account: Option<super::AwsAccountScope>,
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
