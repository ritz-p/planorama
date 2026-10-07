use crate::model::{ArchitectureGraph, TerraformGraph};
mod associations;
mod containment;
mod data;
pub mod diagnostics;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    // Fixed order: direct/spanning/indirect containment, relationship lowering,
    // then data-source visibility. Lowering must see all original consumers.
    let contained = TerraformGraph {
        graph: containment::infer(raw),
        attributes: raw.attributes.clone(),
        graph_references: raw.graph_references.clone(),
    };
    ArchitectureGraph(data::visible(associations::lower(&contained)))
}
