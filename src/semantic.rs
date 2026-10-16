use crate::model::{ArchitectureGraph, TerraformPlan};
mod input;
pub(crate) use input::Input;
#[cfg(test)]
pub(crate) use tests::base_graph;
pub mod cross_state;
pub mod diagnostics;
pub(crate) mod relationships;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/semantic/resolution.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "../tests/unit/semantic/eventbridge.rs"]
mod eventbridge_tests;

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
    graph.coverage = crate::coverage::Coverage::collect(plan, &graph);
    ArchitectureGraph(graph)
}
