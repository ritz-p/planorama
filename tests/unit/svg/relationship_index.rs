use super::*;
use crate::{plan, semantic, svg};

fn graph() -> Graph {
    semantic::transform(&plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap())
        .0
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
    assert_eq!(layout.bounds, Layout::new(&graph).bounds);
}
