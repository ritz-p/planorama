use crate::model::{ArchitectureGraph, TerraformGraph};
mod associations;
mod containment;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    let contained = TerraformGraph {
        graph: containment::infer(raw),
        attributes: raw.attributes.clone(),
    };
    ArchitectureGraph(associations::lower(&contained))
}
