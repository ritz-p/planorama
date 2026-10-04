use super::*;
use crate::{
    model::{Action, Edge, EntityMode, Node},
    plan, semantic, svg,
};

fn fixture() -> Graph {
    let raw = plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap();
    semantic::transform(&raw).0
}

#[test]
fn high_degree_ports_expand_headers_and_remain_distinct_inside_parent_bounds() {
    for role in [ResourceRole::Container, ResourceRole::Node] {
        for count in [56, 57, 100] {
            let mut graph = fixture();
            for (index, node) in graph.nodes.iter_mut().enumerate() {
                node.role = match index {
                    0 => ResourceRole::Container,
                    _ => role,
                };
            }
            graph.edges = [1, 2]
                .map(|child| Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((0, child))
                })
                .into();
            graph.edges.extend((0..count).map(|index| Edge {
                kind: EdgeKind::Association,
                change: Some(crate::model::EdgeChange {
                    address: format!("test.relationship{index}"),
                    action: Action::Create,
                }),
                ..Edge::from((1, 2))
            }));
            let layout = Layout::new(&graph);
            for node in [1, 2] {
                assert!(layout.header_heights[node] >= count + 41);
            }
            let paths = &layout.paths[2..];
            let sources: BTreeSet<_> = paths.iter().map(|path| path[0].y).collect();
            let targets: BTreeSet<_> = paths.iter().map(|path| path.last().unwrap().y).collect();
            assert_eq!(sources.len(), count);
            assert_eq!(targets.len(), count);
            let output = svg::render(&graph, &layout);
            if role == ResourceRole::Node {
                assert!(output.contains(&format!(
                    "width=\"320\" height=\"{}\"",
                    layout.header_heights[1]
                )));
            }
            verify(&graph);
            for index in (2..graph.edges.len()).step_by(2) {
                let edge = &mut graph.edges[index];
                std::mem::swap(&mut edge.from, &mut edge.to);
            }
            let layout = Layout::new(&graph);
            let mut ports = [BTreeSet::new(), BTreeSet::new()];
            for (edge, path) in graph.edges[2..].iter().zip(&layout.paths[2..]) {
                assert!(ports[edge.from - 1].insert(path[0].y));
                assert!(ports[edge.to - 1].insert(path.last().unwrap().y));
            }
            verify(&graph);
        }
    }
}

#[test]
fn reciprocal_relationships_and_self_loops_use_distinct_incoming_and_outgoing_ports() {
    for role in [ResourceRole::Container, ResourceRole::Node] {
        let mut node = fixture()
            .nodes
            .into_iter()
            .find(|node| node.role == ResourceRole::Container)
            .unwrap();
        let mut nodes = Vec::new();
        for index in 0..3 {
            node.address = format!("test.peer{index}");
            node.role = match index {
                2 => ResourceRole::Container,
                _ => role,
            };
            nodes.push(node.clone());
        }
        let edges = [
            (0, 1, Action::Create),
            (1, 0, Action::Update),
            (0, 0, Action::Delete),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (from, to, action))| Edge {
            from,
            to,
            kind: EdgeKind::Association,
            change: Some(crate::model::EdgeChange {
                address: format!("test.relationship{index}"),
                action,
            }),
        })
        .collect();
        let graph = Graph { nodes, edges };
        let layout = Layout::new(&graph);
        assert_ne!(layout.paths[0].first(), layout.paths[1].last());
        assert_ne!(layout.paths[0].last(), layout.paths[1].first());
        let reversed: Vec<_> = layout.paths[1].iter().rev().copied().collect();
        assert_ne!(layout.paths[0], reversed);
        assert_ne!(layout.paths[2].first(), layout.paths[2].last());
        let output = svg::render(&graph, &layout);
        for (index, action) in ["create", "update", "delete"].iter().enumerate() {
            assert!(output.contains(&format!("association; {action}: test.relationship{index}")));
            assert!(output.contains(&format!("marker-end=\"url(#arrow-{action})\"")));
        }
        verify(&graph);
    }
}

#[test]
fn nesting_hides_ancestor_references_but_keeps_sibling_and_external_connections() {
    let mut graph = fixture();
    let parent = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_subnet")
        .unwrap();
    let child = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_instance")
        .unwrap();
    let root = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_vpc")
        .unwrap();
    let sibling = graph.nodes.len();
    let mut node = graph.nodes[child].clone();
    node.address = "aws_instance.sibling".into();
    graph.nodes.push(node.clone());
    node.address = "aws_instance.external".into();
    graph.nodes.push(node);
    graph.edges.extend([
        Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, sibling))
        },
        Edge::from((child, root)),
        Edge::from((child, sibling)),
        Edge::from((child, sibling + 1)),
    ]);
    let original = graph.clone();
    let layout = Layout::new(&graph);
    let paths = &layout.paths[layout.paths.len() - 4..];
    assert!(paths[0].is_empty());
    assert!(paths[1].is_empty());
    assert!(!paths[2].is_empty());
    assert!(!paths[3].is_empty());
    assert_eq!(graph, original);
    verify(&graph);
}

#[test]
fn parallel_association_changes_have_distinct_ports_and_paths() {
    let raw = plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
    let mut graph = semantic::transform(&raw).0;
    let mut other = graph.edges[0].clone();
    let change = other.change.as_mut().unwrap();
    change.address = "aws_route_table_association.other".into();
    change.action = Action::Update;
    graph.edges.push(other);
    let layout = Layout::new(&graph);
    assert_ne!(layout.paths[0], layout.paths[1]);
    assert_ne!(layout.paths[0].first(), layout.paths[1].first());
    assert_ne!(layout.paths[0].last(), layout.paths[1].last());
    let output = svg::render(&graph, &layout);
    for expected in [
        "association; create: aws_route_table_association.private",
        "association; update: aws_route_table_association.other",
        "marker-end=\"url(#arrow-create)\"",
        "marker-end=\"url(#arrow-update)\"",
    ] {
        assert!(output.contains(expected));
    }
    verify(&graph);
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.origin.x < b.origin.x + b.width
        && b.origin.x < a.origin.x + a.width
        && a.origin.y < b.origin.y + b.height
        && b.origin.y < a.origin.y + a.height
}

fn verify(graph: &Graph) {
    let layout = Layout::new(graph);
    for (edge, path) in graph
        .edges
        .iter()
        .zip(&layout.paths)
        .filter(|(_, path)| !path.is_empty())
    {
        for (node, point) in [
            (edge.from, path.first().unwrap()),
            (edge.to, path.last().unwrap()),
        ] {
            let bounds = layout
                .containers
                .iter()
                .find(|(index, _)| *index == node)
                .map(|(_, bounds)| *bounds)
                .unwrap_or(Bounds {
                    origin: layout.positions[node],
                    width: NODE_WIDTH,
                    height: layout.header_heights[node],
                });
            assert_eq!(point.x, bounds.origin.x + bounds.width);
            assert!(point.y > bounds.origin.y && point.y < bounds.origin.y + bounds.height);
        }
    }
    for path in layout.paths.iter().filter(|path| !path.is_empty()) {
        assert_eq!(path[0].y, path[1].y);
        assert!(path[0].x < path[1].x);
        let last = path.len() - 1;
        assert_eq!(path[last].y, path[last - 1].y);
        assert!(path[last].x < path[last - 1].x);
    }
    for &(parent, bounds) in &layout.containers {
        assert!(bounds.origin.x + bounds.width <= layout.width);
        assert!(bounds.origin.y + bounds.height <= layout.height);
        for (child, parent_index) in layout.parents.iter().enumerate() {
            if *parent_index != Some(parent) {
                continue;
            }
            let child_bounds = layout
                .containers
                .iter()
                .find(|(node, _)| *node == child)
                .map(|(_, bounds)| *bounds)
                .unwrap_or(Bounds {
                    origin: layout.positions[child],
                    width: NODE_WIDTH,
                    height: layout.header_heights[child],
                });
            assert!(child_bounds.origin.x >= bounds.origin.x + PADDING);
            assert!(
                child_bounds.origin.y >= bounds.origin.y + layout.header_heights[parent] + PADDING
            );
            assert!(
                child_bounds.origin.x + child_bounds.width + PADDING
                    <= bounds.origin.x + bounds.width
            );
            assert!(
                child_bounds.origin.y + child_bounds.height + PADDING
                    <= bounds.origin.y + bounds.height
            );
        }
    }
    for (i, &origin) in layout.positions.iter().enumerate() {
        let a = Bounds {
            origin,
            width: NODE_WIDTH,
            height: layout.header_heights[i],
        };
        for (j, &other) in layout.positions.iter().enumerate().skip(i + 1) {
            assert!(!overlaps(
                a,
                Bounds {
                    origin: other,
                    height: layout.header_heights[j],
                    ..a
                }
            ));
        }
        for path in &layout.paths {
            for pair in path.windows(2) {
                assert!(pair[0].x == pair[1].x || pair[0].y == pair[1].y);
                assert!(!routing::crosses(pair[0], pair[1], a));
            }
        }
    }
    assert_eq!(
        svg::render(graph, &layout),
        svg::render(graph, &Layout::new(graph))
    );
}

#[test]
fn relationships_anchor_to_outer_frames_for_nested_peer_and_external_containers() {
    for mode in [EntityMode::Managed, EntityMode::Data] {
        let mut graph = fixture();
        let root = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_vpc")
            .unwrap();
        let subnet = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_subnet")
            .unwrap();
        let peer = graph.nodes.len();
        let mut node = graph.nodes[root].clone();
        node.address = "aws_vpc.peer".into();
        node.mode = mode;
        graph.nodes.push(node);
        let card = graph.nodes.len();
        let mut node = graph
            .nodes
            .iter()
            .find(|node| node.resource_type == "aws_instance")
            .unwrap()
            .clone();
        node.address = "aws_instance.external".into();
        graph.nodes.push(node);
        graph
            .edges
            .extend([(root, peer), (peer, subnet), (subnet, card), (card, root)].map(Edge::from));
        let layout = Layout::new(&graph);
        assert!(
            layout.paths[layout.paths.len() - 4..]
                .iter()
                .all(|path| !path.is_empty())
        );
        verify(&graph);
    }
}

#[test]
fn nested_vpc_subnet_ec2_reserve_space_and_route_extra_dependencies() {
    let graph = fixture();
    verify(&graph);
    let layout = Layout::new(&graph);
    assert_eq!(layout.containers.len(), 2);
    assert_eq!(layout.parents.iter().flatten().count(), 2);
    assert_eq!(
        layout.paths.iter().filter(|path| path.is_empty()).count(),
        3
    );
}

#[test]
fn peer_containers_and_cross_module_children_remain_separate_and_identifiable() {
    let mut graph = fixture();
    for node in &mut graph.nodes {
        if node.resource_type == "aws_instance" {
            node.module = "module.compute".into();
        }
    }
    graph.nodes.push(Node {
        address: "aws_vpc.peer".into(),
        resource_type: "aws_vpc".into(),
        module: "module.network".into(),
        action: Action::Create,
        mode: EntityMode::Managed,
        role: ResourceRole::Container,
    });
    graph.edges.push(Edge::from((graph.nodes.len() - 1, 0)));
    verify(&graph);
    let layout = Layout::new(&graph);
    let peer = layout
        .containers
        .iter()
        .find(|(node, _)| *node == graph.nodes.len() - 1)
        .unwrap()
        .1;
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        if edge.from == graph.nodes.len() - 1 || edge.to == graph.nodes.len() - 1 {
            continue;
        }
        for pair in path.windows(2) {
            assert!(!routing::crosses(pair[0], pair[1], peer));
        }
    }
    for &(node, bounds) in &layout.containers {
        if node != graph.nodes.len() - 1 {
            assert!(!overlaps(peer, bounds));
        }
    }
    assert_eq!(layout.bands.len(), 2);
    assert!(svg::render(&graph, &layout).contains("module.compute"));
}

#[test]
fn ambiguous_parents_and_cycles_keep_visible_edges_instead_of_nesting() {
    let mut graph = fixture();
    for node in &mut graph.nodes {
        node.role = ResourceRole::Container;
    }
    graph.edges = vec![
        Edge::from((0, 1)),
        Edge::from((1, 0)),
        Edge::from((0, 2)),
        Edge::from((1, 2)),
    ];
    for edge in &mut graph.edges {
        edge.kind = EdgeKind::Containment;
    }
    let layout = Layout::new(&graph);
    assert!(layout.parents.iter().all(Option::is_none));
    assert!(layout.paths.iter().all(|path| !path.is_empty()));
    verify(&graph);
}
