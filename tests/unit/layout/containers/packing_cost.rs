use super::*;

#[test]
fn equal_size_packings_prefer_nearby_peers_and_hubs() {
    let sizes = vec![(100, 80); 4];
    let near = vec![
        Point { x: 0, y: 0 },
        Point { x: 140, y: 0 },
        Point { x: 0, y: 120 },
        Point { x: 140, y: 120 },
    ];
    let far = vec![near[0], near[3], near[2], near[1]];
    assert!(estimate(&[(0, 1)], &sizes, &near) < estimate(&[(0, 1)], &sizes, &far));
    let peers = vec![(0, 1), (0, 2), (0, 3), (0, 1)];
    assert!(estimate(&peers, &sizes, &near) < estimate(&peers, &sizes, &far));
}

#[test]
fn crossing_estimate_uses_router_penalty_and_ignores_shared_endpoints() {
    let sizes = vec![(100, 80); 4];
    let offsets = vec![
        Point { x: 0, y: 0 },
        Point { x: 140, y: 0 },
        Point { x: 0, y: 120 },
        Point { x: 140, y: 120 },
    ];
    assert_eq!(
        estimate(&[(0, 3), (1, 2)], &sizes, &offsets),
        520 + CROSSING_COST
    );
    assert_eq!(estimate(&[(0, 1), (2, 3)], &sizes, &offsets), 280);
    assert_eq!(estimate(&[(0, 1), (0, 3)], &sizes, &offsets), 400);
    assert_eq!(
        estimate(&[(0, 3), (1, 2), (1, 2)], &sizes, &offsets),
        780 + CROSSING_COST * 2
    );
}

#[test]
fn estimated_distance_uses_dynamic_card_centers() {
    let offsets = vec![Point { x: 0, y: 0 }, Point { x: 140, y: 0 }];
    assert_eq!(estimate(&[(0, 1)], &[(100, 80), (100, 240)], &offsets), 220);
    let aligned = vec![Point { x: 0, y: 80 }, offsets[1]];
    assert_eq!(estimate(&[(0, 1)], &[(100, 80), (100, 240)], &aligned), 140);
}

#[test]
fn packing_bounds_shape_even_when_relationship_weight_is_large() {
    for count in [8, 16, 32, 64] {
        let sizes: Vec<_> = (0..count)
            .map(|node| (100 + node % 3 * 40, 80 + node % 5 * 30))
            .collect();
        let columns = vec![(0..count).collect()];
        let mut baseline = vec![Point { x: 0, y: 0 }; count];
        let old = super::super::pack(&columns, &[], &sizes, &mut baseline, 40);
        let relationships: Vec<_> = (1..count)
            .flat_map(|node| std::iter::repeat_n((0, node), 20))
            .collect();
        let mut positions = baseline.clone();
        let new = super::super::pack(&columns, &relationships, &sizes, &mut positions, 40);
        let span = |(width, height): (usize, usize)| (width * 10).max(height * 17);
        assert!(span(new) * 10 <= span(old) * 11);
        assert!(new.0 * new.1 * 4 <= old.0 * old.1 * 5);
        assert!(
            estimate(&relationships, &sizes, &positions)
                <= estimate(&relationships, &sizes, &baseline)
        );
        for a in 0..count {
            assert!(positions[a].x + sizes[a].0 <= new.0);
            assert!(positions[a].y + sizes[a].1 <= new.1);
            for b in a + 1..count {
                assert!(
                    positions[a].x + sizes[a].0 + 40 <= positions[b].x
                        || positions[b].x + sizes[b].0 + 40 <= positions[a].x
                        || positions[a].y + sizes[a].1 + 40 <= positions[b].y
                        || positions[b].y + sizes[b].1 + 40 <= positions[a].y
                );
            }
        }
    }
}

#[test]
fn hub_packings_choose_shorter_relationships_and_survive_reordering() {
    for count in [5, 11, 19] {
        let sizes: Vec<_> = (0..count)
            .map(|node| (100 + node % 3 * 40, 80 + node % 5 * 30))
            .collect();
        let columns = vec![(0..count).collect()];
        let mut baseline = vec![Point { x: 0, y: 0 }; count];
        let old = super::super::pack(&columns, &[], &sizes, &mut baseline, 40);
        let relationships: Vec<_> = (1..count).map(|node| (0, node)).collect();
        let mut positions = baseline.clone();
        let new = super::super::pack(&columns, &relationships, &sizes, &mut positions, 40);
        assert_ne!(old, new);
        assert!(
            estimate(&relationships, &sizes, &positions)
                < estimate(&relationships, &sizes, &baseline)
        );
        let reversed_sizes: Vec<_> = sizes.into_iter().rev().collect();
        let reversed_columns = vec![(0..count).rev().collect()];
        let reversed_edges: Vec<_> = relationships
            .into_iter()
            .rev()
            .map(|(a, b)| (count - 1 - b, count - 1 - a))
            .collect();
        let mut reversed = vec![Point { x: 0, y: 0 }; count];
        assert_eq!(
            super::super::pack(
                &reversed_columns,
                &reversed_edges,
                &reversed_sizes,
                &mut reversed,
                40
            ),
            new
        );
        assert_eq!(positions, reversed.into_iter().rev().collect::<Vec<_>>());
    }
}

#[test]
fn bounded_crossing_sample_is_independent_of_edge_order() {
    let count = 600;
    let sizes = vec![(100, 80); count];
    let offsets: Vec<_> = (0..count)
        .map(|node| Point {
            x: node % 2 * 1000,
            y: node / 2 * 120,
        })
        .collect();
    let mut edges: Vec<_> = (0..count / 2)
        .map(|node| (node * 2, count - 1 - node * 2))
        .collect();
    let expected = estimate(&edges, &sizes, &offsets);
    edges.reverse();
    assert_eq!(estimate(&edges, &sizes, &offsets), expected);
}
