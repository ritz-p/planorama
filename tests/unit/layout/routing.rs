use super::*;
use crate::model::{Action, Node};
use std::collections::BTreeSet;

#[test]
fn fan_out_and_fan_in_share_trunks_and_preserve_endpoints() {
    for (edges, ranks) in [
        (vec![(0, 1), (0, 2), (0, 3)], vec![0, 1, 1, 1]),
        (vec![(0, 3), (1, 3), (2, 3)], vec![0, 0, 0, 1]),
    ] {
        let graph = Graph {
            nodes: (0..4)
                .map(|i| Node {
                    address: format!("test.n{i}"),
                    resource_type: "test".into(),
                    module: "root".into(),
                    action: Action::Create,
                    mode: crate::model::EntityMode::Managed,
                    role: crate::model::ResourceRole::Unknown,
                })
                .collect(),
            edges: edges.into_iter().map(crate::model::Edge::from).collect(),
        };
        let initial: Vec<_> = (0..4)
            .map(|i| Point {
                x: 60 + ranks[i] * 420,
                y: 208 + i * 144,
            })
            .collect();
        let mut positions = initial.clone();
        let routed = route(&graph, &ranks, &mut positions, &[192, 800]);
        let xs: BTreeSet<_> = routed
            .paths
            .iter()
            .flat_map(|path| path.windows(2))
            .filter(|s| s[0].x == s[1].x && s[0].y != s[1].y)
            .map(|s| s[0].x)
            .collect();
        assert_eq!(xs.len(), 1);
        assert_eq!(routed.paths.len(), graph.edges.len());
        assert_eq!(routed.junctions.len(), 2);
        for (path, edge) in routed.paths.iter().zip(&graph.edges) {
            let (source, target) = edge.endpoints();
            assert_eq!(
                path[0],
                Point {
                    x: positions[source].x + NODE_WIDTH,
                    y: positions[source].y + NODE_HEIGHT / 2
                }
            );
            assert_eq!(
                *path.last().unwrap(),
                Point {
                    x: positions[target].x,
                    y: positions[target].y + NODE_HEIGHT / 2
                }
            );
        }
        let mut again = initial;
        let repeated = route(&graph, &ranks, &mut again, &[192, 800]);
        assert_eq!(routed.paths, repeated.paths);
        assert_eq!(routed.junctions, repeated.junctions);
    }
}

#[test]
fn unrelated_edges_cannot_reuse_an_occupied_bundle_trunk() {
    let mut graph = Graph {
        nodes: (0..5)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
                mode: crate::model::EntityMode::Managed,
                role: crate::model::ResourceRole::Unknown,
            })
            .collect(),
        edges: vec![(0, 2), (0, 3), (1, 4)]
            .into_iter()
            .map(crate::model::Edge::from)
            .collect(),
    };
    let ranks = [0, 0, 1, 1, 1];
    let mut positions = vec![
        Point { x: 60, y: 208 },
        Point { x: 60, y: 352 },
        Point { x: 480, y: 208 },
        Point { x: 480, y: 496 },
        Point { x: 480, y: 640 },
    ];
    let routed = route(&graph, &ranks, &mut positions, &[192, 800]);
    let vertical_x = |path: &[Point]| {
        path.windows(2)
            .find(|s| s[0].x == s[1].x && s[0].y != s[1].y)
            .unwrap()[0]
            .x
    };
    assert_ne!(vertical_x(&routed.paths[1]), vertical_x(&routed.paths[2]));
    graph.edges.swap(1, 2);
    let interleaved = route(&graph, &ranks, &mut positions, &[192, 800]);
    assert_ne!(
        vertical_x(&interleaved.paths[1]),
        vertical_x(&interleaved.paths[2])
    );
    assert_eq!(routed.paths[0], interleaved.paths[0]);
    assert_eq!(routed.paths[1], interleaved.paths[2]);
}

#[test]
fn long_edges_use_separate_channels_after_gutter_expansion() {
    let graph = Graph {
        nodes: (0..18)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
                mode: crate::model::EntityMode::Managed,
                role: crate::model::ResourceRole::Unknown,
            })
            .collect(),
        edges: (0..8)
            .map(|i| (i, 15 - i))
            .chain([(0, 16), (0, 17)])
            .map(crate::model::Edge::from)
            .collect(),
    };
    let ranks: Vec<_> = (0..18).map(|i| i / 8).collect();
    let initial: Vec<_> = (0..18)
        .map(|i| Point {
            x: 60 + i / 8 * 420,
            y: 208 + i % 8 * 144,
        })
        .collect();
    let mut positions = initial.clone();
    let routed = route(&graph, &ranks, &mut positions, &[192, 1360]);
    assert!(positions[8].x - positions[0].x > 420);
    let bridge_y: Vec<_> = routed.paths[8..]
        .iter()
        .map(|path| {
            path.windows(2)
                .find(|s| s[0].y == s[1].y && s[0].x.abs_diff(s[1].x) > NODE_WIDTH)
                .unwrap()[0]
                .y
        })
        .collect();
    assert_ne!(bridge_y[0], bridge_y[1]);
    assert!(routed.paths.iter().flatten().all(|p| p.x < routed.width));
    let mut again = initial;
    assert_eq!(
        routed.paths,
        route(&graph, &ranks, &mut again, &[192, 1360]).paths
    );
}

#[test]
fn eight_overlapping_edges_get_distinct_lanes_inside_the_gutter() {
    let graph = Graph {
        nodes: (0..16)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
                mode: crate::model::EntityMode::Managed,
                role: crate::model::ResourceRole::Unknown,
            })
            .collect(),
        edges: (0..8)
            .map(|i| crate::model::Edge::from((i, 15 - i)))
            .collect(),
    };
    let ranks: Vec<_> = (0..16).map(|i| i / 8).collect();
    let initial: Vec<_> = (0..16)
        .map(|i| Point {
            x: 60 + i / 8 * 420,
            y: 208 + i % 8 * 144,
        })
        .collect();
    let mut positions = initial.clone();
    let routed = route(&graph, &ranks, &mut positions, &[192, 1360]);
    let xs: BTreeSet<_> = routed
        .paths
        .iter()
        .flat_map(|path| path.windows(2))
        .filter(|s| s[0].x == s[1].x && s[0].y != s[1].y)
        .map(|s| s[0].x)
        .collect();
    assert_eq!(xs.len(), 8);
    assert!(positions[8].x - positions[0].x > 420);
    assert!(
        xs.iter()
            .all(|&x| x > positions[0].x + NODE_WIDTH && x < positions[8].x)
    );
    assert!(routed.paths.iter().flatten().all(|p| p.x < routed.width));
    let mut again = initial;
    assert_eq!(
        routed.paths,
        route(&graph, &ranks, &mut again, &[192, 1360]).paths
    );
}
