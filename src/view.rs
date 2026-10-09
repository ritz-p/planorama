//! Immutable projection of a complete architecture into a user-selected view.
//! No Terraform parsing or semantic inference occurs in this layer.
mod filter;
use crate::model::{ArchitectureGraph, Graph, cross_state::Architecture};
pub use filter::Options;
use std::ops::Deref;

/// Owned projected graph; construction is restricted to this projection stage.
pub struct ViewGraph(ArchitectureGraph);
impl Deref for ViewGraph {
    type Target = Graph;
    fn deref(&self) -> &Graph {
        &self.0
    }
}

/// State identities and cross-state provenance survive projection together.
pub struct ViewArchitecture(Architecture);
impl Deref for ViewArchitecture {
    type Target = Architecture;
    fn deref(&self) -> &Architecture {
        &self.0
    }
}

pub fn single(graph: &ArchitectureGraph, options: &Options) -> Result<ViewGraph, String> {
    filter::single(graph, options).map(ViewGraph)
}
pub fn apply(architecture: &Architecture, options: &Options) -> Result<ViewArchitecture, String> {
    filter::apply(architecture, options).map(ViewArchitecture)
}

#[test]
fn projection_keeps_native_relationship_endpoints_and_source_immutable() {
    let plan = crate::plan::parse(include_str!("../tests/fixtures/containment-plan.json")).unwrap();
    let architecture = crate::semantic::transform(&plan);
    let original = architecture.clone();
    let full = single(&architecture, &Options::default()).unwrap();
    assert_eq!(*full, architecture.0);
    let focused = single(
        &architecture,
        &Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let ids: std::collections::BTreeSet<_> = focused.entities().map(|entity| &entity.id).collect();
    assert!(
        focused
            .nodes
            .iter()
            .any(|node| node.resource_type == "aws_vpc")
    );
    for relationship in &focused.relationships {
        assert!(ids.contains(&relationship.from) && ids.contains(&relationship.to));
        assert!(architecture.relationships.contains(relationship));
    }
    assert_eq!(architecture, original);
}
