use super::*;
use crate::model::{Action, Node};

#[test]
fn numbered_cycles_reserve_both_terminal_clearances_between_flat_rows() {
    use crate::layout::{Layout, relationship_markers};
    use crate::model::{Directionality, Edge, ResourceRole};
    for direction in [Directionality::Directed, Directionality::Undirected] {
        let mut graph = crate::semantic::transform(
            &crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap(),
        )
        .0;
        for node in &mut graph.nodes {
            node.role = ResourceRole::Node;
        }
        graph.edges[0].change.as_mut().unwrap().directionality = direction;
        let (from, to) = graph.edges[0].endpoints();
        graph.edges.push(Edge::from((to, from)));
        let ranks = crate::layout::rank::compute(&graph);
        assert_eq!(ranks[from], ranks[to]);
        let clearance = relationship_markers::clearance(&graph);
        let required = clearance
            + if direction == Directionality::Undirected {
                clearance
            } else {
                16
            };
        for height in [super::super::NODE_HEIGHT, 180] {
            let placed = place_with_heights(&graph, &ranks, &vec![height; graph.nodes.len()]);
            assert_eq!(placed.positions[from].x, placed.positions[to].x);
            assert!(placed.positions[from].y.abs_diff(placed.positions[to].y) >= height + required);
        }
        let layout = Layout::new(&graph);
        let path = &layout.paths[0];
        let source = layout.bounds[from];
        let target = layout.bounds[to];
        let (start_y, end_y) = if source.origin.y < target.origin.y {
            (source.origin.y + source.height, target.origin.y)
        } else {
            (source.origin.y, target.origin.y + target.height)
        };
        assert_eq!(path[0].y, start_y);
        assert_eq!(path.last().unwrap().y, end_y);
    }
}

#[test]
fn long_edges_order_through_three_ranks_deterministically() {
    let graph = Graph {
        relationships: Vec::new(),
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
                provider_configuration: None,
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
