use super::*;

#[test]
fn reciprocal_container_connections_keep_distinct_final_ports() {
    use crate::model::{Edge, EdgeKind, ResourceRole};
    let mut graph = graph(2, vec![]);
    graph.nodes[0].role = ResourceRole::Container;
    graph.edges = vec![
        Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((0, 1))
        },
        Edge {
            kind: EdgeKind::Connection,
            ..Edge::from((0, 1))
        },
        Edge {
            kind: EdgeKind::Connection,
            ..Edge::from((1, 0))
        },
    ];
    let layout = Layout::new(&graph);
    assert_ne!(
        layout.paths[1].first(),
        layout.paths[2].last(),
        "container port: {:?}",
        layout.paths
    );
    assert_ne!(
        layout.paths[1].last(),
        layout.paths[2].first(),
        "child port: {:?}",
        layout.paths
    );
}

#[test]
fn refined_flat_bundle_keeps_opposite_role_port() {
    let mut graph = graph(6, vec![(0, 1), (4, 1), (5, 1), (1, 2), (1, 3), (0, 4)]);
    graph.edges[0].kind = crate::model::EdgeKind::Connection;
    graph.edges[1].kind = crate::model::EdgeKind::Association;
    let mut layout = Layout::new(&graph);
    layout.bounds[0].origin.y = 400;
    layout.bounds[1].origin.y = 400;
    layout.bounds[2].origin.y = 200;
    layout.bounds[3].origin.y = 600;
    layout.bounds[4].origin.y = 100;
    layout.bounds[5].origin.y = 700;
    layout.bounds[1].height = 121;
    let routed = routing::route_selected(
        &graph,
        &rank::compute(&graph),
        &mut layout.bounds,
        &[80, 180, 380, 580, 680, 880],
        true,
        Some(&[[Side::Right; 2]; 6]),
    );
    layout.paths = routed.paths;
    layout.junctions = routed.junctions;
    layout.width = routed.width;
    layout.height = layout
        .bounds
        .iter()
        .map(|b| b.origin.y + b.height + 80)
        .max()
        .unwrap();
    layout.header_heights = layout.bounds.iter().map(|b| b.height).collect();
    layout.bands.clear();
    assert_ne!(
        layout.paths[0].last(),
        layout.paths[3].first(),
        "{:?}",
        layout.paths
    );
    let target = layout.paths[0].last().unwrap();
    let source = layout.paths[3][0];
    assert!(target.y.abs_diff(source.y) >= 16);
    assert_eq!(layout.paths[3].first(), layout.paths[4].first());
    assert!(!routing_shared::slots::opposite_roles_collide(
        &graph,
        &layout.paths
    ));
    let rendered = crate::svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    let translated_y = |node: roxmltree::Node| {
        node.ancestors()
            .filter_map(|parent| parent.attribute("transform"))
            .map(|value| {
                value
                    .trim_start_matches("translate(")
                    .trim_end_matches(')')
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            })
            .sum::<usize>()
    };
    let legend = document
        .descendants()
        .find(|node| node.attribute("id") == Some("relationship-legend"))
        .unwrap();
    let legend_rect = legend
        .children()
        .find(|node| node.has_tag_name("rect"))
        .unwrap();
    let legend_bottom = translated_y(legend_rect)
        + legend_rect
            .attribute("height")
            .unwrap()
            .parse::<usize>()
            .unwrap();
    let architecture = document
        .descendants()
        .find(|node| node.attribute("data-architecture") == Some("true"))
        .unwrap();
    let offset = translated_y(architecture);
    assert!(
        layout
            .bounds
            .iter()
            .all(|bounds| offset + bounds.origin.y >= legend_bottom + 16)
    );
    assert!(
        layout
            .paths
            .iter()
            .flatten()
            .all(|point| offset + point.y >= legend_bottom + 16)
    );
    assert_eq!(
        document
            .root_element()
            .attribute("height")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        layout.height + offset
    );
    let snapshot = "examples/port-collisions.svg";
    if std::env::var_os("UPDATE_PORT_SNAPSHOT").is_some() {
        std::fs::write(snapshot, &rendered).unwrap();
    }
    assert_eq!(rendered, std::fs::read_to_string(snapshot).unwrap());
}

#[test]
fn mixed_topologies_keep_distinct_final_ports_under_reordering() {
    use crate::model::{Edge, EdgeKind, ResourceRole};
    let mut random = 19u64;
    for case in 0..256 {
        let count = 3 + case % 5;
        let mut graph = graph(count, vec![]);
        if case % 2 == 0 {
            graph.nodes[0].role = ResourceRole::Container;
            graph.edges.extend((1..count).map(|child| Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((0, child))
            }));
        }
        for _ in 0..(2 + case % 11) {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let from = (random >> 32) as usize % count;
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let to = (random >> 32) as usize % count;
            if from == to {
                continue;
            }
            let kind = [
                EdgeKind::Dependency,
                EdgeKind::Association,
                EdgeKind::Connection,
            ][(random >> 20) as usize % 3];
            let edge = Edge {
                from,
                to,
                kind,
                change: None,
            };
            if !graph.edges.contains(&edge) {
                graph.edges.push(edge);
            }
        }
        let layout = Layout::new(&graph);
        for (a, first) in graph.edges.iter().enumerate() {
            for (b, second) in graph.edges.iter().enumerate() {
                if a != b
                    && first.from == second.to
                    && !layout.paths[a].is_empty()
                    && !layout.paths[b].is_empty()
                {
                    assert_ne!(
                        layout.paths[a].first(),
                        layout.paths[b].last(),
                        "case={case} edges={:?} paths={:?}",
                        graph.edges,
                        layout.paths
                    );
                }
            }
        }
        if case % 8 == 0 {
            let expected = layout.paths;
            graph.nodes.reverse();
            for edge in &mut graph.edges {
                edge.from = count - 1 - edge.from;
                edge.to = count - 1 - edge.to;
            }
            graph.edges.reverse();
            let reordered = Layout::new(&graph);
            assert_eq!(
                expected,
                reordered.paths.into_iter().rev().collect::<Vec<_>>(),
                "case={case}"
            );
        }
    }
}

#[test]
fn mixed_numbered_relationships_keep_final_port_spacing_flat_and_nested() {
    use crate::model::{Directionality, Edge, EdgeChange, EdgeKind, ResourceRole};
    for nested in [false, true] {
        let mut graph = graph(5, vec![(1, 0), (0, 1), (2, 0), (0, 3), (3, 0)]);
        for (index, edge) in graph.edges.iter_mut().enumerate() {
            edge.kind = [
                EdgeKind::Dependency,
                EdgeKind::Association,
                EdgeKind::Connection,
            ][index % 3];
            if index % 2 == 1 {
                edge.change = Some(EdgeChange {
                    address: format!("test.link{index}"),
                    action: Action::Create,
                    directionality: Directionality::Undirected,
                    previous_address: None,
                    metadata: Default::default(),
                });
            }
        }
        if nested {
            graph.nodes[4].role = ResourceRole::Container;
            graph.edges.extend((0..4).map(|child| Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((4, child))
            }));
        }
        graph.relationships = graph
            .edges
            .iter()
            .filter_map(|edge| {
                let change = edge.change.as_ref()?;
                Some(crate::model::ArchitectureRelationship::new(
                    graph.nodes[edge.from].entity.id.clone(),
                    graph.nodes[edge.to].entity.id.clone(),
                    edge.kind,
                    false,
                    vec![crate::model::RelationshipProvenance::Resource {
                        source: crate::model::TerraformEntityId {
                            address: change.address.clone(),
                            deposed_key: None,
                        },
                        change: change.clone(),
                    }],
                ))
            })
            .collect();
        let layout = Layout::new(&graph);
        assert!(!routing_shared::slots::opposite_roles_collide(
            &graph,
            &layout.paths
        ));
        let numbered = relationship_markers::ends(&graph);
        let mut ports = Vec::new();
        for (index, (edge, path)) in graph.edges.iter().zip(&layout.paths).enumerate() {
            if path.len() < 2 {
                continue;
            }
            ports.push((edge.from, path[0], numbered[index][0]));
            ports.push((edge.to, *path.last().unwrap(), numbered[index][1]));
        }
        for (i, &(node, a, numbered_a)) in ports.iter().enumerate() {
            for &(other, b, numbered_b) in &ports[i + 1..] {
                if node != other || a == b {
                    continue;
                }
                let bounds = layout.bounds[node];
                if a.x == b.x && (a.x == bounds.origin.x || a.x == bounds.right()) {
                    let spacing = match (numbered_a, numbered_b) {
                        (true, true) => 24,
                        (false, false) => 16,
                        _ => 20,
                    };
                    assert!(a.y.abs_diff(b.y) >= spacing, "{a:?} {b:?}");
                }
            }
        }
        let expected = layout.paths;
        graph.nodes.reverse();
        for edge in &mut graph.edges {
            edge.from = 4 - edge.from;
            edge.to = 4 - edge.to;
        }
        graph.edges.reverse();
        assert_eq!(
            expected,
            Layout::new(&graph)
                .paths
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
        );
    }
}
