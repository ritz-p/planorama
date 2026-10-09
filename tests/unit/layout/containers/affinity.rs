use super::*;
use crate::layout::metrics::measure;

fn dense() -> Graph {
    semantic::transform(
        &plan::parse(include_str!("../../../fixtures/spanning-dense-plan.json")).unwrap(),
    )
    .0
}

#[test]
fn different_spanning_targets_do_not_share_bundle_segments() {
    let raw = plan::parse(include_str!("../../../fixtures/multi-container-plan.json")).unwrap();
    let graph = semantic::transform(&raw).0;
    let layout = Layout::new(&graph);
    let groups = affinity::groups(&graph, &layout.parents);
    assert_eq!(groups.len(), 2);
    for &first in &groups[0].edges {
        for &second in &groups[1].edges {
            assert_eq!(
                measure(&[layout.paths[first].clone(), layout.paths[second].clone()])
                    .overlap_distance,
                0,
                "different targets share a connection segment: {:?} / {:?}; junctions {:?}",
                layout.paths[first],
                layout.paths[second],
                layout.junctions
            );
        }
    }
    verify(&graph);
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    let other = Layout::new(&reordered);
    assert_eq!(
        layout.paths,
        other.paths.into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(layout.junctions, other.junctions);
}

#[test]
fn three_subnet_alb_stays_in_vpc_and_moves_to_the_source_median() {
    let graph = dense();
    let original = graph.clone();
    let before = place_with_affinity(&graph, false);
    let layout = Layout::new(&graph);
    let group = affinity::groups(&graph, &layout.parents).remove(0);
    let mut centers: Vec<_> = group
        .sources
        .iter()
        .map(|&s| layout.positions[s].y + layout.bounds[s].height / 2)
        .collect();
    centers.sort_unstable();
    let center = centers[1];
    let distance = |l: &Layout<'_>| {
        (l.positions[group.target].y + l.bounds[group.target].height / 2).abs_diff(center)
    };
    assert!(distance(&layout) < distance(&before));
    assert_eq!(distance(&layout), 0);
    assert_eq!(layout.parents[group.target], before.parents[group.target]);
    assert_eq!(layout.bounds.len(), graph.nodes.len());
    assert_eq!(graph, original);
    verify(&graph);
    assert!(
        !layout.junctions.is_empty(),
        "expected a shared connection trunk"
    );
    let old = measure(&before.paths);
    let new = measure(&layout.paths);
    eprintln!("spanning before {old:?}; after {new:?}");
    assert!(new.total_path_length < old.total_path_length);
    assert!(new.crossing_count <= old.crossing_count);
    assert!(layout.width * layout.height <= before.width * before.height);
    let output = svg::render(&graph, &layout);
    assert_eq!(output.matches("data-edge-kind=\"connection\"").count(), 3);
    assert_eq!(output.matches("<g id=\"resource-").count(), 5);
    for &index in &group.edges {
        let edge = &graph.edges[index];
        let path = &layout.paths[index];
        let source = layout.bounds[edge.from];
        assert_eq!(path[0].x, source.right());
        assert!(path[0].y < source.origin.y + layout.header_heights[edge.from]);
        assert_eq!(path.last().unwrap().x, layout.bounds[edge.to].origin.x);
        assert!(output.contains(&format!(
            "{} → {} (connection)",
            graph.nodes[edge.from].address, graph.nodes[edge.to].address
        )));
        for pair in path.windows(2) {
            assert!(
                !routing::obstacles(&layout, edge)
                    .iter()
                    .any(|&b| routing::crosses(pair[0], pair[1], b))
            );
        }
    }
}

#[test]
fn affinity_and_bundles_survive_node_and_edge_reordering() {
    let graph = dense();
    let layout = Layout::new(&graph);
    let mut reversed = graph.clone();
    let count = reversed.nodes.len();
    reversed.nodes.reverse();
    reversed.edges.reverse();
    for edge in &mut reversed.edges {
        edge.from = count - 1 - edge.from;
        edge.to = count - 1 - edge.to;
    }
    let other = Layout::new(&reversed);
    assert_eq!(
        layout.bounds,
        other.bounds.into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(
        layout.paths,
        other.paths.into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(layout.junctions, other.junctions);
}

#[test]
fn two_subnet_alb_and_ecs_use_the_same_generic_affinity_rule() {
    for resource_type in ["aws_lb", "aws_ecs_service"] {
        let mut input: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/spanning-dense-plan.json"))
                .unwrap();
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["address"] != "aws_subnet.c");
        input["resource_changes"][3]["type"] = serde_json::json!(resource_type);
        input["resource_changes"][3]["address"] = serde_json::json!(format!("{resource_type}.app"));
        let resources = input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap();
        resources.retain(|n| n["address"] != "aws_subnet.c");
        resources[2]["address"] = serde_json::json!(format!("{resource_type}.app"));
        let subnets = serde_json::json!({"references": ["aws_subnet.a.id", "aws_subnet.b.id"]});
        resources[2]["expressions"] = if resource_type == "aws_lb" {
            serde_json::json!({"subnets": subnets})
        } else {
            serde_json::json!({"network_configuration": [{"subnets": subnets}]})
        };
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let layout = Layout::new(&graph);
        let group = affinity::groups(&graph, &layout.parents).remove(0);
        assert_eq!(group.sources.len(), 2);
        assert!(!layout.junctions.is_empty());
        assert!(
            group
                .edges
                .iter()
                .all(|&i| layout.paths[i].last().unwrap().x == layout.positions[group.target].x)
        );
    }
}

#[test]
fn wrapping_keeps_related_containers_consecutive_and_uses_safe_fallbacks() {
    let mut graph = dense();
    let parent = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_vpc")
        .unwrap();
    let template = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_subnet")
        .unwrap()
        .clone();
    for name in ["aa", "bb", "cc"] {
        let index = graph.nodes.len();
        graph.nodes.push(Node {
            entity: crate::model::ArchitectureEntity::terraform(crate::model::TerraformEntityId {
                address: format!("aws_subnet.{name}"),
                deposed_key: None,
            }),
            deposed_key: None,
            previous_address: None,
            metadata: Default::default(),
            address: format!("aws_subnet.{name}"),
            ..template.clone()
        });
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, index))
        });
    }
    let layout = Layout::new(&graph);
    let group = affinity::groups(&graph, &layout.parents).remove(0);
    let xs: BTreeSet<_> = group
        .sources
        .iter()
        .map(|&n| layout.positions[n].x)
        .collect();
    assert_eq!(xs.len(), 1, "related containers should wrap together");
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        for pair in path.windows(2) {
            assert!(
                !routing::obstacles(&layout, edge)
                    .iter()
                    .any(|&b| routing::crosses(pair[0], pair[1], b))
            );
        }
    }
    let parents = layout.parents;
    for edge in &mut graph.edges {
        if edge.kind == EdgeKind::Connection {
            edge.change = Some(crate::model::EdgeChange {
                previous_address: None,
                metadata: Default::default(),
                address: "test.changed".into(),
                action: Action::Update,
            });
        }
    }
    assert!(affinity::groups(&graph, &parents).is_empty());
}
