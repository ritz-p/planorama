use crate::model::{ArchitectureGraph, TerraformGraph};

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(raw: &TerraformGraph) -> ArchitectureGraph {
    ArchitectureGraph(raw.graph.clone())
}
