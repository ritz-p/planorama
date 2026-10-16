pub(crate) mod card;
mod filter;
pub(crate) mod geometry;
pub(crate) mod references;
pub(crate) mod routes;
pub(crate) mod scope;
pub(crate) mod spatial;
pub(crate) mod text;
use crate::model::architecture::{Architecture, ArchitectureGraph, Graph};
pub use filter::Options;
use std::ops::Deref;

pub struct ViewGraph(ArchitectureGraph);
impl Deref for ViewGraph {
    type Target = Graph;
    fn deref(&self) -> &Graph {
        &self.0
    }
}

pub struct ViewArchitecture(Architecture);
impl ViewArchitecture {
    pub fn relationships(&self) -> &[crate::model::architecture::CrossStateEdge] {
        &self.0.relationships
    }
}
impl Deref for ViewArchitecture {
    type Target = Architecture;
    fn deref(&self) -> &Architecture {
        &self.0
    }
}

pub fn single(graph: &ArchitectureGraph, options: &Options) -> Result<ViewGraph, String> {
    let _timing = crate::timing::span(
        "selection_projection",
        &[("nodes", graph.nodes.len()), ("edges", graph.edges.len())],
    );
    filter::single(graph, options).map(ViewGraph)
}
pub fn apply(architecture: &Architecture, options: &Options) -> Result<ViewArchitecture, String> {
    let _timing = crate::timing::span(
        "selection_projection",
        &[("states", architecture.states.len())],
    );
    filter::apply(architecture, options).map(ViewArchitecture)
}

#[cfg(test)]
#[path = "../tests/unit/view.rs"]
mod tests;
