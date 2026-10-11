use crate::model::{ArchitectureGraph, TerraformPlan};
mod input;
pub(crate) use input::Input;
#[cfg(test)]
pub(crate) use input::base_graph;
pub mod cross_state;
pub mod diagnostics;
pub(crate) mod relationships;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/semantic/resolution.rs"]
mod resolution_tests;

pub(crate) fn transform(plan: &TerraformPlan) -> ArchitectureGraph {
    let raw = Input::new(plan);
    let mut graph = crate::provider::transform(raw);
    graph.relationships = relationships::collect(&graph);
    ArchitectureGraph(graph)
}
