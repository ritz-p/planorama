use super::*;
use crate::model::{Action, Node};

fn graph(count: usize, edges: Vec<(usize, usize)>) -> Graph {
    Graph {
        nodes: (0..count)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
            })
            .collect(),
        edges,
    }
}

#[test]
fn only_semantically_compatible_stars_are_grouped() {
    let fan_out = graph(4, vec![(0, 1), (0, 2), (0, 3)]);
    let bundles = Bundles::new(&fan_out, &[0, 1, 1, 1]);
    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles.membership, [Some(0); 3]);
    let fan_in = graph(4, vec![(0, 3), (1, 3), (2, 3)]);
    assert_eq!(
        Bundles::new(&fan_in, &[0, 0, 0, 1]).membership,
        [Some(0); 3]
    );
    let mesh = graph(4, vec![(0, 2), (0, 3), (1, 2), (1, 3)]);
    assert_eq!(Bundles::new(&mesh, &[0, 0, 1, 1]).len(), 0);
}

#[test]
fn long_reverse_same_rank_and_cross_module_edges_are_not_bundled() {
    let mut graph = graph(3, vec![(0, 1), (0, 2)]);
    for ranks in [[0, 2, 2], [1, 0, 0], [0, 0, 0]] {
        assert_eq!(Bundles::new(&graph, &ranks).len(), 0);
    }
    graph.nodes[1].module = "module.other".into();
    graph.nodes[2].module = "module.other".into();
    assert_eq!(Bundles::new(&graph, &[0, 1, 1]).len(), 0);
}

#[test]
fn independent_stars_do_not_share_membership() {
    let graph = graph(6, vec![(0, 2), (0, 3), (1, 4), (1, 5)]);
    let bundles = Bundles::new(&graph, &[0, 0, 1, 1, 1, 1]);
    assert_eq!(bundles.len(), 2);
    assert_eq!(bundles.membership, [Some(0), Some(0), Some(1), Some(1)]);
}
