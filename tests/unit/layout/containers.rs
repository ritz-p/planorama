use super::*;
use crate::{
    model::{Action, Edge, EntityMode, Node},
    plan, semantic, svg,
};

fn fixture() -> Graph {
    let raw = plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap();
    semantic::transform(&raw).0
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.origin.x < b.origin.x + b.width
        && b.origin.x < a.origin.x + a.width
        && a.origin.y < b.origin.y + b.height
        && b.origin.y < a.origin.y + a.height
}

fn verify(graph: &Graph) {
    let layout = Layout::new(graph);
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
                    height: NODE_HEIGHT,
                });
            assert!(child_bounds.origin.x >= bounds.origin.x + PADDING);
            assert!(child_bounds.origin.y >= bounds.origin.y + NODE_HEIGHT + PADDING);
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
            height: NODE_HEIGHT,
        };
        for &other in &layout.positions[i + 1..] {
            assert!(!overlaps(a, Bounds { origin: other, ..a }));
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
fn nested_vpc_subnet_ec2_reserve_space_and_route_extra_dependencies() {
    let graph = fixture();
    verify(&graph);
    let layout = Layout::new(&graph);
    assert_eq!(layout.containers.len(), 2);
    assert_eq!(layout.parents.iter().flatten().count(), 2);
    assert_eq!(
        layout.paths.iter().filter(|path| path.is_empty()).count(),
        2
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
