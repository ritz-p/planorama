use super::{ArchitectureId, EdgeChange, EdgeKind, TerraformEntityId};
#[cfg(test)]
#[path = "../../tests/unit/model/relationships.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipProvenance {
    Resource {
        source: TerraformEntityId,
        change: EdgeChange,
    },
    Reference {
        from: TerraformEntityId,
        to: TerraformEntityId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArchitectureRelationship {
    pub from: ArchitectureId,
    pub to: ArchitectureId,
    pub kind: EdgeKind,
    pub inferred: bool,
    pub provenance: Vec<RelationshipProvenance>,
}

impl ArchitectureRelationship {
    pub fn derived_containment(&self) -> bool {
        self.kind == EdgeKind::Containment
            && self.inferred
            && !self.provenance.is_empty()
            && !self.provenance.iter().any(|proof| match proof {
                RelationshipProvenance::Reference { from, to } => {
                    ArchitectureId::terraform(from) == self.from
                        && ArchitectureId::terraform(to) == self.to
                }
                RelationshipProvenance::Resource { .. } => false,
            })
    }

    pub fn new(
        from: ArchitectureId,
        to: ArchitectureId,
        kind: EdgeKind,
        inferred: bool,
        mut provenance: Vec<RelationshipProvenance>,
    ) -> Self {
        provenance.sort();
        provenance.dedup();
        Self {
            from,
            to,
            kind,
            inferred,
            provenance,
        }
    }
}
