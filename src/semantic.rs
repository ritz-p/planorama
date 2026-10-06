use crate::model::{ArchitectureGraph, TerraformGraph};
mod associations;
mod containment;
mod data;
pub mod diagnostics;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    let contained = TerraformGraph {
        graph: containment::infer(raw),
        attributes: raw.attributes.clone(),
    };
    ArchitectureGraph(data::visible(associations::lower(&contained)))
}
