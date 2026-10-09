use super::*;
use crate::model::{Action, Node};

#[test]
fn long_edges_order_through_three_ranks_deterministically() {
    let graph = Graph {
        components: Vec::new(),
        checks: Vec::new(),
        status: Default::default(),
        nodes: (0..4)
            .map(|i| Node {
                entity: crate::model::ArchitectureEntity::terraform(
                    crate::model::TerraformEntityId {
                        address: format!("test.n{i}"),
                        deposed_key: None,
                    },
                ),
                deposed_key: None,
                previous_address: None,
                metadata: Default::default(),
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                provider: crate::model::ProviderIdentity::inferred("test"),
                module: "root".into(),
                action: Action::Create,
                mode: crate::model::EntityMode::Managed,
                role: crate::model::ResourceRole::Unknown,
            })
            .collect(),
        edges: vec![(0, 3), (1, 2)]
            .into_iter()
            .map(crate::model::Edge::from)
            .collect(),
    };
    let ranks = [0, 0, 3, 3];
    let placement = place(&graph, &ranks);
    assert_eq!(placement.positions.len(), 4);
    assert_eq!(
        placement.positions[0].y < placement.positions[1].y,
        placement.positions[3].y < placement.positions[2].y
    );
    for _ in 0..10 {
        assert_eq!(placement.positions, place(&graph, &ranks).positions);
    }
    assert_eq!(placement.positions[2].x, 60 + 3 * COLUMN_STEP);
}
