use crate::model::{ArchitectureGraph, TerraformGraph};
mod containment;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    ArchitectureGraph(containment::infer(raw))
}
