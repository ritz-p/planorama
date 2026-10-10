use super::TerraformEntity;
pub(crate) use super::cross_state::{Architecture, CrossStateEdge, CrossStateProvenance, Endpoint};
pub(crate) use super::{
    Action, ArchitectureGraph, AttributePathStep, ChangeMetadata, CheckResult, Edge, EdgeKind,
    EntityMode, Graph, Node, RelationshipProvenance, ResourceRole, StateId,
};
use std::fmt::Write;
#[cfg(test)]
#[path = "../../tests/unit/model/architecture.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TerraformEntityId {
    pub address: String,
    pub deposed_key: Option<String>,
}

impl From<&TerraformEntity> for TerraformEntityId {
    fn from(entity: &TerraformEntity) -> Self {
        Self {
            address: entity.address.clone(),
            deposed_key: entity.deposed_key.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArchitectureId(String);

impl ArchitectureId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub(super) fn terraform(source: &TerraformEntityId) -> Self {
        Self(format!(
            "terraform:{}-{}",
            hex(&source.address),
            match &source.deposed_key {
                None => "current".into(),
                Some(key) => format!("deposed-{}", hex(key)),
            }
        ))
    }
    pub fn synthetic(kind: &str, key: &str) -> Self {
        Self(format!("synthetic:{}-{}", hex(kind), hex(key)))
    }
}

fn hex(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        write!(out, "{byte:02x}").unwrap();
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArchitectureEntityKind {
    Terraform,
    Synthetic,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArchitectureEntity {
    pub id: ArchitectureId,
    pub kind: ArchitectureEntityKind,
    pub provenance: Vec<TerraformEntityId>,
}

impl ArchitectureEntity {
    pub fn terraform(source: TerraformEntityId) -> Self {
        Self {
            id: ArchitectureId::terraform(&source),
            kind: ArchitectureEntityKind::Terraform,
            provenance: vec![source],
        }
    }
    pub fn synthetic(kind: &str, key: &str, mut provenance: Vec<TerraformEntityId>) -> Self {
        provenance.sort();
        provenance.dedup();
        Self {
            id: ArchitectureId::synthetic(kind, key),
            kind: ArchitectureEntityKind::Synthetic,
            provenance,
        }
    }
}

impl Graph {
    pub fn derived_containment(&self, parent: usize, child: usize) -> bool {
        self.relationships.iter().any(|relationship| {
            relationship.from == self.nodes[parent].entity.id
                && relationship.to == self.nodes[child].entity.id
                && relationship.derived_containment()
        })
    }

    pub fn entities(&self) -> impl Iterator<Item = &ArchitectureEntity> {
        self.nodes
            .iter()
            .map(|n| &n.entity)
            .chain(self.components.iter().map(|c| &c.entity))
    }
}
