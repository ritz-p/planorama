//! Complete architecture inference before user selection; delegates concrete provider rules.
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

pub(crate) fn transform(plan: &TerraformPlan) -> ArchitectureGraph {
    let raw = Input::new(plan);
    // Fixed order: direct/spanning/indirect containment, security-group connections, relationship lowering,
    // then data-source visibility. Lowering must see all original consumers.
    let mut graph = crate::provider::transform(raw);
    graph.relationships = relationships::collect(&graph);
    ArchitectureGraph(graph)
}
