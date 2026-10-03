use crate::model::{ArchitectureGraph, TerraformGraph};
mod associations;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    ArchitectureGraph(associations::lower(raw))
}
