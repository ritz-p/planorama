use super::*;
use crate::model::{Action, Node};
use std::collections::BTreeSet;

#[test]
fn eight_overlapping_edges_get_distinct_lanes_inside_the_gutter() {
    let graph = Graph {
        nodes: (0..16)
            .map(|i| Node {
                address: format!("test.n{i}"),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
            })
            .collect(),
        edges: (0..8).map(|i| (i, 15 - i)).collect(),
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
