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
    let _timing = crate::timing::span(
        "semantic_transform",
        &[
            ("resources", plan.nodes.len()),
            ("references", plan.edges.len()),
        ],
    );
    let raw = Input::new(plan);
    let deployments = relationships::deployments(&raw.nodes);
    let mut graph = crate::provider::transform(raw);
    graph.relationships = relationships::collect(&graph, &deployments);
    ArchitectureGraph(graph)
}
