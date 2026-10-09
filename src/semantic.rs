use crate::model::{ArchitectureGraph, TerraformGraph};
mod associations;
mod containment;
mod data;
pub mod diagnostics;
mod security_groups;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    // Fixed order: direct/spanning/indirect containment, security-group connections, relationship lowering,
    // then data-source visibility. Lowering must see all original consumers.
    let contained = TerraformGraph {
        graph: security_groups::infer(raw, containment::infer(raw)),
        attributes: raw.attributes.clone(),
        graph_references: raw.graph_references.clone(),
        drift: raw.drift.clone(),
        relevant_attributes: raw.relevant_attributes.clone(),
    };
    ArchitectureGraph(data::visible(associations::lower(&contained)))
}
