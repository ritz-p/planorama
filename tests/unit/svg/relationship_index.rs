use super::*;
use crate::{plan, semantic, svg};

fn graph() -> Graph {
    semantic::transform(&plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap())
        .0
}

#[test]
fn dense_targets_keep_distinct_numbers_readable() {
    for kind in [EdgeKind::Association, EdgeKind::Connection] {
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
        for number in 0..12 {
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
        let layout = Layout::new(&graph);
        let index = Index::new(&graph, &layout);
        assert_eq!(index.markers.len(), 12);
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
    let svg = index.render(0);
    let document = roxmltree::Document::parse(&svg).unwrap();
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
        assert!(100 + width <= index.width);
        assert!(line.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < index.height());
    }
    assert_eq!(index.height(), 60 + lines.len() * 20 + 8);
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
fn markers_stay_on_the_line_near_the_destination_in_each_direction() {
    use crate::layout::Point;
    let graph = graph();
    for (start, end) in [
        (Point { x: 2000, y: 2000 }, Point { x: 2300, y: 2000 }),
        (Point { x: 2300, y: 2000 }, Point { x: 2000, y: 2000 }),
        (Point { x: 2000, y: 2000 }, Point { x: 2000, y: 2300 }),
        (Point { x: 2000, y: 2300 }, Point { x: 2000, y: 2000 }),
    ] {
        let mut layout = Layout::new(&graph);
        layout.paths[0] = vec![start, end];
        layout.width = 2600;
        layout.height = 2600;
        let index = Index::new(&graph, &layout);
        let bounds = index.markers[0].bounds;
        let center = Point {
            x: bounds.origin.x + bounds.width / 2,
            y: bounds.origin.y + bounds.height / 2,
        };
        assert_eq!(center.x == end.x, start.x == end.x);
        assert_eq!(center.y == end.y, start.y == end.y);
        let distance = center.x.abs_diff(end.x) + center.y.abs_diff(end.y);
        assert_eq!(
            distance,
            if start.x == end.x {
                bounds.height / 2 + 10
            } else {
                bounds.width / 2 + 10
            }
        );
        assert_eq!(index.width, layout.width);
    }
}

#[test]
fn routed_badges_fit_terminal_segments_and_avoid_other_paths() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/aws-relationships-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
        include_str!("../../../examples/terraform-large/plan.json"),
    ] {
        let graph = semantic::transform(&plan::parse(fixture).unwrap());
        let layout = Layout::new(&graph);
        assert_marker_routes(&graph, &layout);
    }
}

fn assert_marker_routes(graph: &Graph, layout: &Layout<'_>) {
    let index = Index::new(graph, layout);
    let edges = index.entries.iter().flat_map(|entry| entry.edges.iter());
    for (marker, &edge) in index.markers.iter().zip(edges) {
        let bounds = marker.bounds;
        let terminal = layout.paths[edge]
            .windows(2)
            .rev()
            .find(|s| s[0] != s[1])
            .unwrap();
        let (before, target) = (terminal[0], terminal[1]);
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
                if other == edge && position + 1 == segments {
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
fn resource_provenance_is_indexed_and_reference_provenance_is_not() {
    let graph = graph();
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.markers.len(), 1);
    let svg = svg::render(&graph, &layout);
    assert!(svg.contains("Relationship resources"));
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
    assert_eq!(empty.height(), 0);
    assert!(!svg::render(&inferred, &layout).contains("Relationship resources"));
    assert_eq!(
        svg::dimensions(&graph, &layout).1 - svg::dimensions(&inferred, &layout).1,
        index.height()
    );
}

#[test]
fn multiple_sources_long_addresses_and_xml_are_preserved_without_values() {
    let mut graph = graph();
    let relationship = graph
        .relationships
        .iter_mut()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap();
    let RelationshipProvenance::Resource { change, .. } = relationship.provenance[0].clone() else {
        panic!()
    };
    let address = format!(
        "module.{}.aws_route_table_association.extra[\"<&>\"]",
        "nested.".repeat(70)
    );
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
}

#[test]
fn numbering_comes_from_sources_and_survives_reordered_edges_and_provenance() {
    let raw = plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap();
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
    assert_eq!(index.markers.len(), 1);
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
