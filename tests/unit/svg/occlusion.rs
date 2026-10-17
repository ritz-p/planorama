use super::*;
use crate::model::{Edge, EdgeKind, ResourceRole};

fn fixture() -> Graph {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.c","type":"test"},{"address":"test.d","type":"test"},{"address":"test.e","type":"test"},{"address":"test.f","type":"test"},{"address":"test.g","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    for node in [0, 1, 3] {
        graph.nodes[node].role = ResourceRole::Container;
    }
    graph.edges = [(0, 1), (1, 2), (1, 3), (3, 4), (0, 5), (0, 6)]
        .map(|pair| Edge {
            kind: EdgeKind::Containment,
            ..Edge::from(pair)
        })
        .to_vec();
    graph
        .edges
        .extend([(5, 6), (2, 4), (5, 4), (1, 6)].map(Edge::from));
    graph
}

#[test]
fn masks_hide_foreign_containers_without_hiding_internal_or_endpoint_relationships() {
    let graph = fixture();
    let layout = Layout::new(&graph);
    let masks = Masks::new(&graph, &layout);
    let group = masks.edges[6].unwrap();
    assert_eq!(masks.groups[group], vec![1]);
    for index in [7, 8] {
        assert!(masks.edges[index].is_none());
    }
    assert!(!masks.groups[masks.edges[9].unwrap()].contains(&1));
    let svg = crate::svg::render(&graph, &layout);
    let xml = roxmltree::Document::parse(&svg).unwrap();
    let id = format!("container-occlusion-{group}");
    let mask = xml
        .descendants()
        .find(|n| n.attribute("id") == Some(id.as_str()))
        .unwrap();
    assert_eq!(mask.attribute("maskUnits"), Some("userSpaceOnUse"));
    let hidden = mask
        .children()
        .find(|n| n.attribute("fill") == Some("black"))
        .unwrap();
    assert_eq!(
        hidden.attribute("x").unwrap().parse::<usize>().unwrap(),
        layout.bounds[1].origin.x - 2
    );
    assert_eq!(
        hidden.attribute("width").unwrap().parse::<usize>().unwrap(),
        layout.bounds[1].width + 4
    );
    let source = crate::svg::identity::resource(&graph.nodes[5]);
    let target = crate::svg::identity::resource(&graph.nodes[6]);
    let path = xml
        .descendants()
        .find(|n| {
            n.has_tag_name("path")
                && n.attribute("data-source") == Some(source.as_str())
                && n.attribute("data-target") == Some(target.as_str())
        })
        .unwrap();
    assert_eq!(path.attribute("mask"), Some(format!("url(#{id})").as_str()));
    let endpoint = xml
        .descendants()
        .find(|n| {
            n.attribute("data-endpoints") == Some("true")
                && n.attribute("data-source") == Some(source.as_str())
        })
        .unwrap();
    assert!(endpoint.attribute("mask").is_none());
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    reordered.nodes.reverse();
    for edge in &mut reordered.edges {
        edge.from = 6 - edge.from;
        edge.to = 6 - edge.to;
    }
    let other = Layout::new(&reordered);
    assert_eq!(
        masks.definitions(&graph, &layout),
        Masks::new(&reordered, &other).definitions(&reordered, &other)
    );
}
