use super::*;
use crate::model::{Action, Node};

fn graph(edges: Vec<(usize, usize)>) -> Graph {
    Graph {
        status: Default::default(),
        nodes: (0..4)
            .map(|i| Node {
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
        edges: edges.into_iter().map(crate::model::Edge::from).collect(),
    }
}

#[test]
fn three_rank_span_has_two_virtual_vertices_and_adjacent_transitions() {
    let graph = graph(vec![(0, 1), (1, 2), (2, 3), (0, 3)]);
    let expanded = Expanded::new(&graph, &[0, 1, 2, 3]);
    assert_eq!(expanded.vertices.len(), 6);
    assert_eq!(expanded.vertices[4].rank, 1);
    assert_eq!(expanded.vertices[5].rank, 2);
    assert!(
        expanded.vertices[4..]
            .iter()
            .all(|node| node.resource.is_none())
    );
    assert_eq!(expanded.outgoing[0], [1, 4]);
    assert_eq!(expanded.outgoing[4], [5]);
    assert_eq!(expanded.outgoing[5], [3]);
    for (source, targets) in expanded.outgoing.iter().enumerate() {
        for &target in targets {
            assert_eq!(
                expanded.vertices[target].rank,
                expanded.vertices[source].rank + 1
            );
            assert!(expanded.incoming[target].contains(&source));
        }
    }
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(
        graph
            .edges
            .iter()
            .map(|edge| edge.endpoints())
            .collect::<Vec<_>>(),
        [(0, 1), (1, 2), (2, 3), (0, 3)]
    );
}

#[test]
fn same_rank_cycles_and_adjacent_edges_need_no_virtual_vertices() {
    let graph = graph(vec![(0, 1), (1, 0), (1, 2), (2, 3)]);
    let expanded = Expanded::new(&graph, &[0, 0, 1, 2]);
    assert_eq!(expanded.vertices.len(), 4);
    assert_eq!(expanded.outgoing[1], [0, 2]);
}

#[test]
fn virtual_vertices_use_the_source_band_without_changing_resource_modules() {
    let mut graph = graph(vec![(0, 3)]);
    graph.nodes[0].module = "module.source".into();
    graph.nodes[3].module = "module.target".into();
    let expanded = Expanded::new(&graph, &[0, 1, 2, 3]);
    assert!(
        expanded.vertices[4..]
            .iter()
            .all(|node| node.module == "module.source")
    );
    assert_eq!(expanded.vertices[3].module, "module.target");
    assert_eq!(expanded.vertices[3].resource, Some(3));
}
