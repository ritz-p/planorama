use super::*;

fn indexed_graph(count: usize) -> Graph {
    let mut graph = crate::semantic::transform(
        &crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap(),
    )
    .0;
    let edge = graph.edges[0].clone();
    let relationship = graph
        .relationships
        .iter()
        .find(|relationship| {
            relationship
                .provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap()
        .clone();
    graph.edges.clear();
    graph.relationships.clear();
    for number in 1..=count {
        let mut edge = edge.clone();
        edge.change.as_mut().unwrap().address = format!("association.test_{number:03}");
        let mut relationship = relationship.clone();
        for p in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { source, change, .. } = p {
                source.address = edge.change.as_ref().unwrap().address.clone();
                *change = edge.change.as_ref().unwrap().clone();
            }
        }
        graph.edges.push(edge);
        graph.relationships.push(relationship);
    }
    graph
}

#[test]
fn marker_approaches_use_actual_longitudinal_and_lateral_dimensions() {
    for (number, width) in [(1, 24), (10, 32), (100, 40)] {
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let port = Port {
                point: Point { x: 200, y: 200 },
                side,
            };
            let terminal = Terminal::marker(number, side);
            let marker = bounds(number, port);
            let corridor = terminal.corridor(port).unwrap();
            let vertical = matches!(side, Side::Top | Side::Bottom);
            assert_eq!(marker.width, width);
            assert_eq!(marker.height, HEIGHT);
            assert_eq!(
                terminal.clearance,
                GAP + (if vertical { HEIGHT } else { width }) + 8
            );
            assert_eq!(terminal.span, (if vertical { width } else { HEIGHT }) + 6);
            assert!(marker.origin.x >= corridor.origin.x);
            assert!(marker.origin.y >= corridor.origin.y);
            assert!(marker.right() <= corridor.right());
            assert!(marker.origin.y + marker.height <= corridor.origin.y + corridor.height);
            if vertical {
                assert_eq!(marker.origin.x - corridor.origin.x, 3);
                assert_eq!(corridor.right() - marker.right(), 3);
                assert_eq!(terminal.clearance, 36);
            } else {
                assert_eq!(marker.origin.y - corridor.origin.y, 3);
                assert_eq!(
                    corridor.origin.y + corridor.height - marker.origin.y - marker.height,
                    3
                );
            }
            let gap = match side {
                Side::Left => port.point.x - marker.right(),
                Side::Right => marker.origin.x - port.point.x,
                Side::Top => port.point.y - marker.origin.y - marker.height,
                Side::Bottom => marker.origin.y - port.point.y,
            };
            assert_eq!(gap, GAP);
            assert!(gap >= 7);
        }
    }
    for clearance in [16, 20] {
        let plain = Terminal::plain(clearance);
        assert!(
            plain
                .corridor(Port {
                    point: Point { x: 200, y: 200 },
                    side: Side::Right,
                })
                .is_none()
        );
    }
}

#[test]
fn unrelated_wide_labels_do_not_expand_a_small_terminal_or_slot() {
    let graph = indexed_graph(100);
    let numbers = numbers(&graph);
    assert_eq!(numbers[0], [Some(1); 2]);
    assert_eq!(numbers[9], [Some(10); 2]);
    assert_eq!(numbers[99], [Some(100); 2]);
    let horizontal = terminals(&graph, 0, numbers[0], [Side::Right, Side::Left]);
    let vertical = terminals(&graph, 99, numbers[99], [Side::Bottom, Side::Top]);
    assert_eq!(horizontal.map(|terminal| terminal.clearance), [42; 2]);
    assert_eq!(vertical.map(|terminal| terminal.clearance), [36; 2]);
    let incident = [(0, true), (9, true), (99, true)];
    let top = ports_with_minimum(&incident, &numbers, 155, Side::Top, 16);
    assert_eq!(top, [35, 69, 111]);
    let side = ports_with_minimum(&incident, &numbers, 113, Side::Left, 16);
    assert_eq!(side, [32, 56, 80]);
}

#[test]
fn numbering_matches_changes_and_groups_sorted_resource_provenance() {
    let mut graph = indexed_graph(2);
    let RelationshipProvenance::Resource { source, change, .. } =
        graph.relationships[0].provenance[0].clone()
    else {
        panic!()
    };
    graph.relationships[0]
        .provenance
        .push(RelationshipProvenance::Resource {
            deployment: None,
            source: TerraformEntityId {
                address: "association.extra".into(),
                ..source
            },
            change,
        });
    let duplicate = graph.relationships[0].provenance[0].clone();
    graph.relationships[0].provenance.push(duplicate);
    let grouped = entries(&graph);
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].sources.len(), 2);
    assert_eq!(grouped[0].sources[0].address, "association.extra");
    let snapshot = |graph: &Graph| {
        graph
            .edges
            .iter()
            .zip(numbers(graph))
            .map(|(edge, numbers)| (edge.change.as_ref().unwrap().address.clone(), numbers))
            .collect::<BTreeMap<_, _>>()
    };
    let original = snapshot(&graph);
    graph.edges.reverse();
    graph.relationships.reverse();
    for relationship in &mut graph.relationships {
        relationship.provenance.reverse();
    }
    assert_eq!(snapshot(&graph), original);
    let mut unmatched = graph.edges[0].clone();
    unmatched.change.as_mut().unwrap().address = "association.unmatched".into();
    graph.edges.push(unmatched);
    assert_eq!(*numbers(&graph).last().unwrap(), [None; 2]);
}

#[test]
fn logical_relationships_do_not_shift_visible_numbers() {
    let mut graph = indexed_graph(2);
    let edge = graph.edges[0].clone();
    let mut hidden = graph.nodes[edge.from].clone();
    hidden.address = "test.logical".into();
    hidden.entity = crate::model::ArchitectureEntity::terraform(TerraformEntityId {
        address: hidden.address.clone(),
        deposed_key: None,
    });
    hidden.placement = Placement::Logical;
    graph.relationships[0].from = hidden.entity.id.clone();
    graph.edges[0].from = graph.nodes.len();
    graph.nodes.push(hidden);
    assert_eq!(numbers(&graph), [[None; 2], [Some(1); 2]]);
    let projected = crate::view::spatial::project(&graph);
    assert_eq!(numbers(&projected.graph), [[Some(1); 2]]);
}

#[test]
fn embedded_directed_routes_reserve_numbered_source_and_target() {
    let graph = crate::semantic::transform(
        &crate::plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap(),
    )
    .0;
    let numbers = numbers(&graph);
    let mut seen = 0;
    for (index, edge) in graph.edges.iter().enumerate() {
        if numbers[index][1].is_none()
            || crate::view::routes::owner(&graph, &graph.nodes[edge.from]).is_none()
        {
            continue;
        }
        assert_eq!(edge.directionality(), Directionality::Directed);
        assert_eq!(numbers[index][0], numbers[index][1]);
        let terminals = terminals(&graph, index, numbers[index], [Side::Right, Side::Top]);
        assert_eq!(
            terminals[0],
            Terminal::marker(numbers[index][0].unwrap(), Side::Right)
        );
        assert_eq!(terminals[1].clearance, 36);
        assert!(terminals[1].span > 0);
        seen += 1;
    }
    assert!(seen > 0);
}

#[test]
fn adjacent_corridors_merge_without_closing_unreserved_gaps() {
    use super::super::{Bounds, Point};
    let mut corridors: Vec<_> = (0..200)
        .map(|i| Bounds {
            origin: Point {
                x: 500,
                y: 100 + i * PORT_SPACING,
            },
            width: 46,
            height: PORT_SPACING,
        })
        .collect();
    let separate = Bounds {
        origin: Point { x: 500, y: 5000 },
        width: 46,
        height: PORT_SPACING,
    };
    corridors.push(separate);
    corridors.reverse();
    let merged = merge_corridors(corridors);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].height, 200 * PORT_SPACING);
    assert_eq!(merged[1], separate);
}
