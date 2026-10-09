use super::*;
use crate::model::{Action, Node};

fn star_paths(count: usize) -> Vec<Vec<Point>> {
    (0..count)
        .map(|y| {
            vec![
                Point { x: 0, y: 0 },
                Point { x: 10, y: 0 },
                Point { x: 10, y },
                Point { x: 20, y },
            ]
        })
        .collect()
}

fn one_bundle(count: usize) -> Bundles {
    Bundles {
        groups: vec![Bundle {
            edges: (0..count).collect(),
            shared: Shared::Source,
        }],
        membership: vec![Some(0); count],
    }
}

#[test]
fn ten_thousand_edge_stars_find_all_branch_points_in_both_directions() {
    let bundles = one_bundle(10_000);
    let mut paths = star_paths(10_000);
    let expected: Vec<_> = (0..9_999).map(|y| Point { x: 10, y }).collect();
    assert_eq!(bundles.junctions(&paths), expected);
    for path in &mut paths {
        path.reverse();
    }
    assert_eq!(bundles.junctions(&paths), expected);
}

#[test]
fn sweep_preserves_junctions_at_touching_intervals_and_horizontal_subdivisions() {
    let paths = vec![
        vec![
            Point { x: 0, y: 10 },
            Point { x: 10, y: 10 },
            Point { x: 10, y: 20 },
            Point { x: 20, y: 20 },
        ],
        vec![
            Point { x: 0, y: 20 },
            Point { x: 10, y: 20 },
            Point { x: 10, y: 30 },
            Point { x: 20, y: 30 },
        ],
        vec![
            Point { x: 0, y: 20 },
            Point { x: 10, y: 20 },
            Point { x: 20, y: 20 },
        ],
    ];
    assert_eq!(
        one_bundle(3).junctions(&paths),
        vec![Point { x: 10, y: 20 }]
    );
    let straight = vec![vec![Point { x: 0, y: 0 }, Point { x: 20, y: 0 }]; 2];
    assert!(one_bundle(2).junctions(&straight).is_empty());
}

fn graph(count: usize, edges: Vec<(usize, usize)>) -> Graph {
    Graph {
        relationships: Vec::new(),
        components: Vec::new(),
        checks: Vec::new(),
        status: Default::default(),
        nodes: (0..count)
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
        edges: edges.into_iter().map(crate::model::Edge::from).collect(),
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

#[test]
fn semantic_edges_and_resource_changes_are_not_bundled() {
    let mut graph = graph(3, vec![(0, 1), (0, 2)]);
    graph.edges[0].kind = EdgeKind::Association;
    assert_eq!(Bundles::new(&graph, &[0, 1, 1]).len(), 0);
    graph.edges[0].kind = EdgeKind::Dependency;
    graph.edges[0].change = Some(crate::model::EdgeChange {
        previous_address: None,
        metadata: Default::default(),
        address: "test.relationship".into(),
        action: Action::Update,
    });
    assert_eq!(Bundles::new(&graph, &[0, 1, 1]).len(), 0);
}
