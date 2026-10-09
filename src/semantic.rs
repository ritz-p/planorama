use crate::model::{ArchitectureGraph, TerraformPlan};
mod input;
use input::Input;
#[cfg(test)]
pub(crate) use input::base_graph;
mod associations;
mod components;
mod containment;
pub mod cross_state;
mod data;
pub mod diagnostics;
pub mod filter;
mod routes;
mod security_groups;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

pub fn transform(plan: &TerraformPlan) -> ArchitectureGraph {
    let raw = Input::new(plan);
    // Fixed order: direct/spanning/indirect containment, security-group connections, relationship lowering,
    // then data-source visibility. Lowering must see all original consumers.
    let graph = security_groups::infer(&raw, containment::infer(&raw));
    let contained = Input { graph, ..raw };
    let mut graph = data::visible(associations::lower(&contained));
    graph.components = components::infer(&contained, &graph);
    ArchitectureGraph(graph)
}
