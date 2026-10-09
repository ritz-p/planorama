use super::*;

#[test]
fn dense_same_rank_and_root_groups_wrap_without_changing_containment() {
    let raw = plan::parse(include_str!(
        "../../../fixtures/dense-architecture-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&raw).0;
    let original = graph.clone();
    let layout = Layout::new(&graph);
    let vpc = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_vpc")
        .unwrap();
    let subnet_x: BTreeSet<_> = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_subnet")
        .map(|(i, _)| {
            assert_eq!(layout.containment.parents[i], Some(vpc));
            layout.positions[i].x
        })
        .collect();
    let root_x: BTreeSet<_> = layout
        .containment
        .parents
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_none())
        .map(|(i, _)| layout.positions[i].x)
        .collect();
    assert!(subnet_x.len() >= 2);
    assert!(root_x.len() >= 2);
    // The old single-column diagram was 2248 px tall. Keep both dimensions
    // bounded rather than merely trading a vertical strip for a horizontal one.
    assert!(layout.height < 1500, "{} x {}", layout.width, layout.height);
    assert!(layout.width < 2500, "{} x {}", layout.width, layout.height);
    assert_eq!(graph, original);
    verify(&graph);
}

#[test]
fn nested_variable_size_groups_keep_final_bounds_and_rank_flow() {
    let mut graph = ranked_fixture();
    graph
        .edges
        .retain(|edge| edge.kind == EdgeKind::Containment);
    graph.nodes[1].role = ResourceRole::Container;
    let template = graph.nodes[2].clone();
    for i in 7..19 {
        graph.nodes.push(Node {
            entity: crate::model::ArchitectureEntity::terraform(crate::model::TerraformEntityId {
                address: format!("test.child{i}"),
                deposed_key: None,
            }),
            deposed_key: None,
            previous_address: None,
            metadata: Default::default(),
            address: format!("test.child{i}"),
            ..template.clone()
        });
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((1, i))
        });
    }
    graph.edges.push(Edge::from((1, 6)));
    let layout = Layout::new(&graph);
    assert!(layout.bounds[1].right() < layout.bounds[6].origin.x);
    assert!(
        (7..19)
            .map(|i| layout.positions[i].x)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    );
    verify(&graph);
    let count = graph.nodes.len();
    let bounds = layout.bounds;
    graph.nodes.reverse();
    graph.edges.reverse();
    for edge in &mut graph.edges {
        edge.from = count - 1 - edge.from;
        edge.to = count - 1 - edge.to;
    }
    let reversed = Layout::new(&graph);
    assert_eq!(
        bounds,
        reversed.bounds.into_iter().rev().collect::<Vec<_>>()
    );
}
