use super::*;
use crate::model::{EdgeKind, RelationshipProvenance};
use crate::{plan, semantic, svg};

fn graph() -> Graph {
    semantic::transform(&plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap())
        .0
}

fn dense_graph(count: usize, kind: EdgeKind) -> Graph {
    let mut graph = graph();
    let edge = graph.edges[0].clone();
    let relationship = graph
        .relationships
        .iter()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap()
        .clone();
    graph.edges.clear();
    graph.relationships.clear();
    for number in 0..count {
        let mut edge = edge.clone();
        edge.kind = kind;
        edge.change.as_mut().unwrap().address = format!("association.test_{number}");
        let mut relationship = relationship.clone();
        relationship.kind = kind;
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { source, change } = provenance {
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
fn association_numbering_preserves_relationship_directions_and_arrows() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/aws-relationships-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
    ] {
        let graph = semantic::transform(&plan::parse(fixture).unwrap()).0;
        let layout = Layout::new(&graph);
        let svg = svg::render(&graph, &layout);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let paths: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
            .collect();
        let edges = graph
            .edges
            .iter()
            .zip(&layout.paths)
            .filter(|(_, p)| !p.is_empty())
            .map(|(e, _)| e);
        for (path, edge) in paths.iter().zip(edges) {
            let directed = edge.directionality() == crate::model::Directionality::Directed;
            assert_eq!(path.attribute("marker-end").is_some(), directed);
            assert_eq!(
                path.attribute("data-directionality"),
                Some(edge.directionality().as_str())
            );
            if edge.kind == EdgeKind::Association {
                assert_eq!(path.attribute("stroke-dasharray"), Some("7 4"));
            }
        }
        let index = Index::new(&graph, &layout);
        for entry in &index.entries {
            let expected: usize = entry
                .edges
                .iter()
                .map(|&e| {
                    if graph.edges[e].kind == EdgeKind::Association
                        || graph.edges[e].directionality()
                            == crate::model::Directionality::Undirected
                    {
                        2
                    } else {
                        1
                    }
                })
                .sum();
            assert_eq!(
                index
                    .markers
                    .iter()
                    .filter(|m| m.number == entry.number)
                    .count(),
                expected
            );
        }
        assert_marker_routes(&graph, &layout);
    }
}

#[test]
fn dense_targets_keep_distinct_numbers_readable() {
    for (kind, directed, count) in [
        (EdgeKind::Association, false, 24),
        (EdgeKind::Association, true, 24),
        (EdgeKind::Connection, false, 24),
        (EdgeKind::Dependency, true, 12),
    ] {
        let mut graph = dense_graph(12, kind);
        if directed {
            for edge in &mut graph.edges {
                edge.change.as_mut().unwrap().directionality =
                    crate::model::Directionality::Directed;
            }
            for relationship in &mut graph.relationships {
                for provenance in &mut relationship.provenance {
                    if let RelationshipProvenance::Resource { change, .. } = provenance {
                        change.directionality = crate::model::Directionality::Directed;
                    }
                }
            }
        }
        let layout = Layout::new(&graph);
        let index = Index::new(&graph, &layout);
        assert_eq!(index.markers.len(), count);
        assert_eq!(index.entries.len(), 12);
        assert_marker_routes(&graph, &layout);
        for (i, marker) in index.markers.iter().enumerate() {
            for other in &index.markers[i + 1..] {
                let (a, b) = (marker.bounds, other.bounds);
                assert!(
                    a.right() + 3 <= b.origin.x
                        || b.right() + 3 <= a.origin.x
                        || a.origin.y + a.height + 3 <= b.origin.y
                        || b.origin.y + b.height + 3 <= a.origin.y,
                    "{kind:?}: {a:?} overlaps {b:?}"
                );
            }
        }
    }
}

#[test]
fn indexed_flat_graphs_preserve_root_and_child_module_bands() {
    let mut graph = dense_graph(12, EdgeKind::Association);
    for node in &mut graph.nodes {
        node.role = crate::model::ResourceRole::Node;
    }
    graph.nodes[0].module = "module.network".into();
    graph.nodes[1].module = "module.application".into();
    let mut root = graph.nodes[0].clone();
    root.module = "root".into();
    root.address = "test.root".into();
    root.entity = crate::model::ArchitectureEntity::terraform(TerraformEntityId {
        address: root.address.clone(),
        deposed_key: None,
    });
    graph.nodes.push(root);
    let layout = Layout::new(&graph);
    assert_eq!(layout.bands.len(), 3);
    assert_eq!(layout.bands[0].label, "root");
    for (node, bounds) in graph.nodes.iter().zip(&layout.bounds) {
        let band = layout
            .bands
            .iter()
            .find(|band| band.label == node.module)
            .unwrap();
        assert!(bounds.origin.y >= band.top + 48);
        assert!(bounds.origin.y + bounds.height <= band.top + band.height);
    }
    assert_marker_routes(&graph, &layout);
}

#[test]
fn one_numbered_port_does_not_expand_every_port_on_a_hub() {
    let mut graph = dense_graph(1, EdgeKind::Association);
    let target = graph.edges[0].to;
    let source = graph.edges[0].from;
    for _ in 0..99 {
        graph.edges.push(crate::model::Edge::from((target, source)));
    }
    let layout = Layout::new(&graph);
    assert!(
        layout.header_heights[target] < 41 + 100 * 24,
        "{}",
        layout.header_heights[target]
    );
    assert_marker_routes(&graph, &layout);
}

#[test]
fn hundreds_of_numbered_relationships_keep_clear_terminal_corridors() {
    let graph = dense_graph(200, EdgeKind::Association);
    let started = std::time::Instant::now();
    let layout = Layout::new(&graph);
    eprintln!("200 numbered relationships: {:?}", started.elapsed());
    assert_eq!(Index::new(&graph, &layout).markers.len(), 400);
    assert_marker_routes(&graph, &layout);
}

#[test]
fn relationship_index_chunks_complete_entries_and_wraps_cards_deterministically() {
    for count in [0usize, 1, 10, 11, 20, 21] {
        let mut graph = dense_graph(count, EdgeKind::Association);
        let snapshot = |graph: &Graph| {
            let layout = Layout::new(graph);
            let index = Index::new(graph, &layout);
            let cards = index.cards();
            assert_eq!(cards.len(), count.div_ceil(10));
            assert_eq!(index.markers.len(), count * 2);
            let full = svg::render(graph, &layout);
            assert_eq!(
                full.matches(">Relationship Resources</text>").count(),
                usize::from(count > 0)
            );
            assert!(!full.contains(">Relationship resources</text>"));
            for (i, card) in cards.iter().enumerate() {
                let doc = roxmltree::Document::parse(&card.content).unwrap();
                let numbers: Vec<_> = doc
                    .descendants()
                    .filter_map(|n| n.attribute("data-relationship-entry"))
                    .map(|n| n.parse::<usize>().unwrap())
                    .collect();
                assert_eq!(
                    numbers,
                    (i * 10 + 1..=(count.min((i + 1) * 10))).collect::<Vec<_>>()
                );
                assert!(!card.content.contains("Relationship resources"));
                assert!(!card.content.contains("Association</text>"));
            }
            let width = cards.first().map_or(360, |card| card.width);
            let area = crate::svg::annotations::Area::new(cards, 2 * width + 100);
            let svg = area.render();
            if count == 0 {
                assert_eq!(area.height, 0);
                assert!(svg.is_empty());
                return svg;
            }
            let document = roxmltree::Document::parse(&svg).unwrap();
            let entries: Vec<_> = document
                .descendants()
                .filter_map(|n| n.attribute("data-relationship-entry"))
                .collect();
            assert_eq!(
                entries,
                (1..=count).map(|i| i.to_string()).collect::<Vec<_>>()
            );
            let rows: std::collections::BTreeSet<_> = document
                .descendants()
                .filter(|n| n.attribute("data-annotation-card").is_some())
                .map(|n| {
                    n.attribute("transform")
                        .unwrap()
                        .split_whitespace()
                        .last()
                        .unwrap()
                })
                .collect();
            assert_eq!(rows.len(), count.div_ceil(20));
            let positions: Vec<_> = document
                .descendants()
                .filter(|n| n.attribute("data-annotation-card").is_some())
                .map(|n| n.attribute("transform").unwrap())
                .collect();
            assert_eq!(positions[0], "translate(40 160)");
            if count > 10 {
                assert_eq!(positions[1], format!("translate({} 160)", width + 60));
            }
            svg
        };
        let expected = snapshot(&graph);
        graph.edges.reverse();
        graph.relationships.reverse();
        assert_eq!(snapshot(&graph), expected);
    }
}

#[test]
fn wide_addresses_wrap_within_the_canvas_and_reserved_height() {
    let mut graph = graph();
    let address = format!(
        "aws_route_table_association.example[\"{}\"]",
        "東京大阪".repeat(80)
    );
    for relationship in &mut graph.relationships {
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { source, .. } = provenance {
                source.address = address.clone();
            }
        }
    }
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let cards = index.cards();
    assert_eq!(cards.len(), 1);
    let svg = &cards[0].content;
    let document = roxmltree::Document::parse(svg).unwrap();
    let lines: Vec<_> = document
        .descendants()
        .filter(|n| n.has_tag_name("tspan"))
        .collect();
    assert_eq!(
        lines.iter().filter_map(|n| n.text()).collect::<String>(),
        address
    );
    for line in &lines {
        let width: usize = line
            .text()
            .unwrap()
            .chars()
            .map(|c| if c.is_ascii() { 8 } else { 16 })
            .sum();
        assert!(
            line.attribute("x").unwrap().parse::<usize>().unwrap() + width <= cards[0].width - 16
        );
        assert!(line.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < cards[0].height);
    }
    assert_eq!(cards[0].height, 44 + lines.len() * 20);
}

#[test]
fn a_long_multisource_tenth_entry_stays_in_the_first_card() {
    let mut graph = dense_graph(11, EdgeKind::Association);
    let relationship = &mut graph.relationships[8];
    let RelationshipProvenance::Resource { change, .. } = relationship.provenance[0].clone() else {
        panic!()
    };
    let address = format!("z.{}.extra", "long_module.".repeat(100));
    relationship
        .provenance
        .push(RelationshipProvenance::Resource {
            source: TerraformEntityId {
                address: address.clone(),
                deposed_key: Some("old-key".into()),
            },
            change,
        });
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let cards = index.cards();
    assert_eq!(cards.len(), 2);
    let first = roxmltree::Document::parse(&cards[0].content).unwrap();
    let tenth = first
        .descendants()
        .find(|n| n.attribute("data-relationship-entry") == Some("10"))
        .unwrap();
    let sources: Vec<_> = tenth
        .descendants()
        .filter(|n| n.attribute("data-source-address").is_some())
        .collect();
    assert_eq!(sources.len(), 2);
    let visible: String = sources[1]
        .children()
        .filter(|n| n.has_tag_name("tspan"))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(visible, format!("{address} (deposed: old-key)"));
    assert!(
        sources[1]
            .children()
            .filter(|n| n.has_tag_name("tspan"))
            .count()
            > 10
    );
    assert!(cards[0].height > cards[1].height);
    assert_eq!(
        first
            .descendants()
            .filter(|n| n.attribute("data-relationship-entry").is_some())
            .count(),
        10
    );
    let second = roxmltree::Document::parse(&cards[1].content).unwrap();
    assert_eq!(
        second
            .descendants()
            .filter_map(|n| n.attribute("data-relationship-entry"))
            .collect::<Vec<_>>(),
        vec!["11"]
    );
}

#[test]
fn marker_borders_match_relationship_actions_and_line_patterns() {
    use crate::model::Action;
    for kind in [
        EdgeKind::Dependency,
        EdgeKind::Association,
        EdgeKind::Connection,
    ] {
        for action in [
            None,
            Some(Action::Create),
            Some(Action::Delete),
            Some(Action::Update),
            Some(Action::Replace),
            Some(Action::Read),
            Some(Action::Unchanged),
            Some(Action::Other),
        ] {
            let mut graph = graph();
            graph.edges[0].kind = kind;
            if let Some(action) = action {
                graph.edges[0].change.as_mut().unwrap().action = action;
            } else {
                graph.edges[0].change = None;
            }
            for relationship in &mut graph.relationships {
                relationship.kind = kind;
                for source in &mut relationship.provenance {
                    if let RelationshipProvenance::Resource { change, .. } = source {
                        if let Some(action) = action {
                            change.action = action;
                        }
                    }
                }
            }
            let svg = svg::render(&graph, &Layout::new(&graph));
            let document = roxmltree::Document::parse(&svg).unwrap();
            let path = document
                .descendants()
                .find(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
                .unwrap();
            let marker = document
                .descendants()
                .find(|n| n.attribute("data-relationship-marker") == Some("1"))
                .unwrap();
            let border = marker.children().find(|n| n.has_tag_name("rect")).unwrap();
            if kind == EdgeKind::Dependency {
                let arrow_id = path
                    .attribute("marker-end")
                    .unwrap()
                    .strip_prefix("url(#")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap();
                let arrow = document
                    .descendants()
                    .find(|n| n.attribute("id") == Some(arrow_id))
                    .unwrap();
                assert_eq!(arrow.attribute("markerUnits"), Some("userSpaceOnUse"));
                let arrow_width: f64 = arrow.attribute("markerWidth").unwrap().parse().unwrap();
                let border_width: f64 = border.attribute("stroke-width").unwrap().parse().unwrap();
                assert!(arrow_width + border_width / 2.0 < 10.0);
            } else {
                assert!(path.attribute("marker-end").is_none());
            }
            let entry = document
                .descendants()
                .find(|n| n.attribute("data-relationship-entry") == Some("1"))
                .unwrap();
            let index_border = entry.children().find(|n| n.has_tag_name("rect")).unwrap();
            for attribute in [
                "stroke",
                "stroke-width",
                "stroke-dasharray",
                "stroke-linecap",
            ] {
                assert_eq!(
                    border.attribute(attribute),
                    path.attribute(attribute),
                    "{kind:?} {action:?}: {attribute}"
                );
                assert_eq!(
                    index_border.attribute(attribute),
                    border.attribute(attribute),
                    "index {kind:?} {action:?}: {attribute}"
                );
            }
        }
    }
}

#[test]
fn markers_stay_on_the_line_near_numbered_ends_in_each_direction() {
    use crate::layout::Point;
    let mut graph = dense_graph(10, EdgeKind::Association);
    for edge in &mut graph.edges {
        edge.change.as_mut().unwrap().directionality = crate::model::Directionality::Directed;
    }
    for relationship in &mut graph.relationships {
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { change, .. } = provenance {
                change.directionality = crate::model::Directionality::Directed;
            }
        }
    }
    for (start, end) in [
        (Point { x: 2000, y: 2000 }, Point { x: 2300, y: 2000 }),
        (Point { x: 2300, y: 2000 }, Point { x: 2000, y: 2000 }),
        (Point { x: 2000, y: 2000 }, Point { x: 2000, y: 2300 }),
        (Point { x: 2000, y: 2300 }, Point { x: 2000, y: 2000 }),
    ] {
        let mut layout = Layout::new(&graph);
        layout.paths.iter_mut().for_each(Vec::clear);
        layout.paths[9] = vec![start, end];
        layout.width = 2600;
        layout.height = 2600;
        let index = Index::new(&graph, &layout);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.markers.len(), 2);
        for (marker, port) in index.markers.iter().zip([end, start]) {
            assert_eq!(marker.number, 10);
            let bounds = marker.bounds;
            assert_eq!(bounds.width, 32);
            let center = Point {
                x: bounds.origin.x + bounds.width / 2,
                y: bounds.origin.y + bounds.height / 2,
            };
            assert_eq!(center.x == port.x, start.x == end.x);
            assert_eq!(center.y == port.y, start.y == end.y);
            let distance = center.x.abs_diff(port.x) + center.y.abs_diff(port.y);
            assert_eq!(
                distance,
                if start.x == end.x {
                    bounds.height / 2 + 10
                } else {
                    bounds.width / 2 + 10
                }
            );
        }
        assert_eq!(index.width, layout.width);
    }
}

#[test]
fn routed_badges_fit_terminal_segments_and_avoid_other_paths() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/aws-relationships-plan.json"),
    ] {
        let graph = semantic::transform(&plan::parse(fixture).unwrap());
        let layout = Layout::new(&graph);
        assert_marker_routes(&graph, &layout);
    }
}

fn assert_marker_routes(graph: &Graph, layout: &Layout<'_>) {
    use crate::layout::{Port, Side, relationship_markers};
    let index = Index::new(graph, layout);
    let shared = relationship_markers::entries(graph);
    for entry in &index.entries {
        assert_eq!(entry.sources, shared[entry.number - 1].sources);
        assert!(
            entry
                .edges
                .iter()
                .all(|edge| shared[entry.number - 1].edges.contains(edge))
        );
    }
    assert!(!index.markers.is_empty());
    let edges = index
        .entries
        .iter()
        .flat_map(|entry| entry.edges.iter())
        .flat_map(|&edge| {
            let mut ends = vec![(edge, false)];
            if graph.edges[edge].kind == EdgeKind::Association
                || graph.edges[edge].directionality() == crate::model::Directionality::Undirected
            {
                ends.push((edge, true));
            }
            ends
        });
    for (marker, (edge, source)) in index.markers.iter().zip(edges) {
        let bounds = marker.bounds;
        let terminal = if source {
            &layout.paths[edge][..2]
        } else {
            layout.paths[edge]
                .windows(2)
                .rev()
                .find(|s| s[0] != s[1])
                .unwrap()
        };
        let (before, target) = if source {
            (terminal[1], terminal[0])
        } else {
            (terminal[0], terminal[1])
        };
        let side = if before.x < target.x {
            Side::Left
        } else if before.x > target.x {
            Side::Right
        } else if before.y < target.y {
            Side::Top
        } else {
            Side::Bottom
        };
        let port = Port {
            point: target,
            side,
        };
        assert_eq!(bounds, relationship_markers::bounds(marker.number, port));
        let reservation = relationship_markers::Terminal::marker(marker.number, side)
            .corridor(port)
            .unwrap();
        assert!(bounds.origin.x >= reservation.origin.x && bounds.right() <= reservation.right());
        assert!(
            bounds.origin.y >= reservation.origin.y
                && bounds.origin.y + bounds.height <= reservation.origin.y + reservation.height
        );
        if before.y == target.y {
            assert_eq!(bounds.origin.y + bounds.height / 2, target.y);
            assert!(
                bounds.origin.x >= before.x.min(target.x)
                    && bounds.right() <= before.x.max(target.x),
                "marker {}: {bounds:?}, terminal {terminal:?}",
                marker.number
            );
        } else {
            assert_eq!(bounds.origin.x + bounds.width / 2, target.x);
            assert!(
                bounds.origin.y >= before.y.min(target.y)
                    && bounds.origin.y + bounds.height <= before.y.max(target.y)
            );
        }
        for (other, path) in layout.paths.iter().enumerate() {
            let segments = path.windows(2).count();
            for (position, segment) in path.windows(2).enumerate() {
                if other == edge
                    && ((source && position == 0) || (!source && position + 1 == segments))
                {
                    continue;
                }
                let (a, b) = (segment[0], segment[1]);
                let crosses = if a.x == b.x {
                    a.x > bounds.origin.x.saturating_sub(2)
                        && a.x < bounds.right() + 2
                        && a.y.max(b.y) > bounds.origin.y.saturating_sub(2)
                        && a.y.min(b.y) < bounds.origin.y + bounds.height + 2
                } else {
                    a.y > bounds.origin.y.saturating_sub(2)
                        && a.y < bounds.origin.y + bounds.height + 2
                        && a.x.max(b.x) > bounds.origin.x.saturating_sub(2)
                        && a.x.min(b.x) < bounds.right() + 2
                };
                assert!(
                    !crosses,
                    "marker {} on edge {edge}: {bounds:?} crosses edge {other} segment {segment:?}",
                    marker.number
                );
            }
        }
    }
}

#[test]
fn missing_paths_keep_the_shared_number_and_its_actual_badge_width() {
    let graph = dense_graph(10, EdgeKind::Association);
    let mut layout = Layout::new(&graph);
    let shared = crate::layout::relationship_markers::entries(&graph);
    for entry in &shared[..9] {
        for &edge in &entry.edges {
            layout.paths[edge].clear();
        }
    }
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].number, 10);
    assert!(
        index
            .markers
            .iter()
            .all(|marker| marker.number == 10 && marker.bounds.width == 32)
    );
    assert!(
        index.cards()[0]
            .content
            .contains("data-relationship-entry=\"10\"")
    );
    assert_marker_routes(&graph, &layout);
}

#[test]
fn embedded_next_hops_share_the_routed_numbering_and_marker_geometry() {
    let graph =
        semantic::transform(&plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap())
            .0;
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let svg = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert!(index.entries.iter().any(|entry| entry.embedded));
    for entry in index.entries.iter().filter(|entry| entry.embedded) {
        assert_eq!(
            index
                .markers
                .iter()
                .filter(|marker| marker.number == entry.number)
                .count(),
            entry.edges.len() * 2
        );
        let number = entry.number.to_string();
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-relationship-entry") == Some(number.as_str()))
                .count(),
            1
        );
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-relationship-marker") == Some(number.as_str()))
                .count(),
            entry.edges.len() * 2
        );
        for &edge in &entry.edges {
            let route = &graph.nodes[graph.edges[edge].from];
            let card = document
                .descendants()
                .find(|node| {
                    node.attribute("data-embedded-route") == Some("true")
                        && node.attribute("data-terraform-address") == Some(route.address.as_str())
                })
                .unwrap();
            let label = format!("Next hop [{}]", entry.number);
            assert!(
                card.descendants()
                    .any(|node| node.text() == Some(label.as_str()))
            );
            assert_eq!(entry.sources.len(), 1);
            assert!(route.entity.provenance.contains(entry.sources[0]));
        }
    }
    assert_marker_routes(&graph, &layout);
}

#[test]
fn resource_provenance_is_indexed_and_reference_provenance_is_not() {
    let graph = graph();
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.markers.len(), 2);
    let svg = svg::render(&graph, &layout);
    assert!(svg.contains("data-relationship-index="));
    assert_eq!(svg.matches(">Relationship Resources</text>").count(), 1);
    assert!(svg.contains("data-relationship-marker=\"1\""));
    assert!(svg.contains("data-relationship-entry=\"1\""));
    assert!(svg.contains(&format!(
        "data-source-address=\"{}\"",
        index.entries[0].sources[0].address
    )));
    let mut inferred = graph.clone();
    inferred.relationships.iter_mut().for_each(|r| {
        r.provenance
            .retain(|p| matches!(p, RelationshipProvenance::Reference { .. }))
    });
    let empty = Index::new(&inferred, &layout);
    assert!(empty.entries.is_empty());
    assert!(empty.cards().is_empty());
    assert!(!svg::render(&inferred, &layout).contains("data-relationship-index="));
    assert_eq!(
        svg::dimensions(&graph, &layout).1 - svg::dimensions(&inferred, &layout).1,
        index.cards()[0].height + 48 + 16 + 24
    );
}

#[test]
fn multiple_sources_long_addresses_and_xml_are_preserved_without_values() {
    let mut graph = graph();
    let first_previous = "aws_route_table_association.old[0]";
    for edge in &mut graph.edges {
        if let Some(change) = &mut edge.change {
            change.previous_address = Some(first_previous.into());
        }
    }
    let relationship = graph
        .relationships
        .iter_mut()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap();
    for provenance in &mut relationship.provenance {
        if let RelationshipProvenance::Resource { change, .. } = provenance {
            change.previous_address = Some(first_previous.into());
        }
    }
    let RelationshipProvenance::Resource { mut change, .. } = relationship.provenance[0].clone()
    else {
        panic!()
    };
    let address = format!(
        "module.{}.aws_route_table_association.extra[\"<&>\"]",
        "nested.".repeat(70)
    );
    let previous = format!(
        "module.{}.aws_route_table_association.old[\"<&>\"]",
        "以前の名前.".repeat(50)
    );
    change.previous_address = Some(previous.clone());
    relationship
        .provenance
        .push(RelationshipProvenance::Resource {
            source: TerraformEntityId {
                address: address.clone(),
                deposed_key: Some("old<&>".into()),
            },
            change,
        });
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries[0].sources.len(), 2);
    let svg = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&svg).unwrap();
    let source = document
        .descendants()
        .find(|n| n.attribute("data-source-address") == Some(&address))
        .unwrap();
    let visible: String = source
        .children()
        .filter(|n| n.has_tag_name("tspan"))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(visible, format!("{address} (deposed: old<&>)"));
    assert!(svg.contains("2 Terraform resources"));
    let height: usize = document
        .root_element()
        .attribute("height")
        .unwrap()
        .parse()
        .unwrap();
    for node in source.children().filter(|n| n.has_tag_name("tspan")) {
        assert!(node.attribute("y").unwrap().parse::<usize>().unwrap() < height);
    }
    let card = &index.cards()[0];
    let doc = roxmltree::Document::parse(&card.content).unwrap();
    let entry = doc
        .descendants()
        .find(|n| n.attribute("data-relationship-entry") == Some("1"))
        .unwrap();
    let texts: Vec<_> = entry
        .children()
        .filter(|n| n.has_tag_name("text"))
        .collect();
    for source in texts
        .iter()
        .filter(|n| n.attribute("data-source-address").is_some())
    {
        let position = texts.iter().position(|n| n == source).unwrap();
        let annotation = texts[position + 1];
        assert_eq!(annotation.attribute("data-move-annotation"), Some("true"));
        let expected = if source.attribute("data-source-address") == Some(address.as_str()) {
            &previous
        } else {
            first_previous
        };
        let lines: Vec<_> = annotation
            .children()
            .filter(|n| n.has_tag_name("tspan"))
            .collect();
        assert_eq!(
            lines.iter().filter_map(|n| n.text()).collect::<String>(),
            format!("Moved from: {expected}")
        );
        assert_eq!(
            annotation
                .children()
                .find(|n| n.has_tag_name("title"))
                .unwrap()
                .text(),
            Some(format!("Moved from: {expected}").as_str())
        );
        for line in lines {
            let width = crate::view::text::width(line.text().unwrap()) * 8;
            assert!(
                line.attribute("x").unwrap().parse::<usize>().unwrap() + width <= card.width - 16
            );
            assert!(line.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < card.height);
        }
    }
    let expected = card.content.clone();
    for relationship in &mut graph.relationships {
        relationship
            .provenance
            .extend(relationship.provenance.clone());
        relationship.provenance.reverse();
    }
    graph.relationships.reverse();
    assert_eq!(
        Index::new(&graph, &Layout::new(&graph)).cards()[0].content,
        expected
    );
}

#[test]
fn numbering_comes_from_sources_and_survives_reordered_edges_and_provenance() {
    let raw = plan::parse(include_str!("../../fixtures/aws-relationships-plan.json")).unwrap();
    let mut graph = semantic::transform(&raw).0;
    let keys = |graph: &Graph| {
        let layout = Layout::new(graph);
        Index::new(graph, &layout)
            .entries
            .iter()
            .map(|e| {
                e.sources
                    .iter()
                    .map(|s| s.address.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let expected = keys(&graph);
    assert!(expected.len() > 1);
    assert!(expected.windows(2).all(|w| w[0] <= w[1]));
    graph.edges.reverse();
    graph.relationships.reverse();
    for relationship in &mut graph.relationships {
        relationship.provenance.reverse();
    }
    assert_eq!(keys(&graph), expected);
}

#[test]
fn marker_boxes_avoid_cards_boundaries_junctions_and_paths() {
    let raw = plan::parse(include_str!("../../fixtures/aws-relationships-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert!(!index.markers.is_empty());
    for marker in &index.markers {
        for (i, bounds) in layout.bounds.iter().enumerate() {
            let header = bounds.header(layout.header_heights[i]);
            assert!(
                marker.bounds.right() <= header.origin.x
                    || marker.bounds.origin.x >= header.right()
                    || marker.bounds.origin.y + marker.bounds.height <= header.origin.y
                    || marker.bounds.origin.y >= header.origin.y + header.height
            );
        }
    }
}

#[test]
fn regional_markers_keep_clear_of_scope_headers_and_frame_edges() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    value["configuration"]["provider_config"] = serde_json::json!({"aws":{"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"ap-northeast-1"}}}});
    for resource in value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        resource["provider_config_key"] = serde_json::json!("aws");
    }
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let layout = Layout::new(&graph);
    assert_eq!(layout.scopes.len(), 1);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.markers.len(), 2);
    let panel = layout.scopes[0].bounds;
    let marker = index.markers[0].bounds;
    assert!(
        marker.origin.y >= panel.origin.y + 36
            || marker.right() < panel.origin.x
            || marker.origin.x > panel.right()
    );
    for x in [panel.origin.x, panel.right()] {
        assert!(
            x < marker.origin.x
                || x > marker.right()
                || marker.origin.y > panel.origin.y + panel.height
                || marker.origin.y + marker.height < panel.origin.y
        );
    }
}
