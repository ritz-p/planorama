use super::*;
use crate::model::{Action, Node};
#[test]
fn cycle_is_condensed_and_dependents_follow() {
    let nodes = (0..4)
        .map(|i| Node {
            deposed_key: None,
            previous_address: None,
            metadata: Default::default(),
            address: i.to_string(),
            resource_type: "test".into(),
            provider: crate::model::ProviderIdentity::inferred("test"),
            module: "root".into(),
            action: Action::Create,
            mode: crate::model::EntityMode::Managed,
            role: crate::model::ResourceRole::Unknown,
        })
        .collect();
    let graph = Graph {
        checks: Vec::new(),
        status: Default::default(),
        nodes,
        edges: vec![(0, 1), (1, 0), (1, 2), (2, 3)]
            .into_iter()
            .map(crate::model::Edge::from)
            .collect(),
    };
    assert_eq!(compute(&graph), vec![0, 0, 1, 2]);
}
