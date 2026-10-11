use super::*;

#[test]
fn hierarchy_preserves_roots_nested_children_ambiguity_and_cycles() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"aws_vpc.a","type":"aws_vpc"},{"address":"aws_vpc.b","type":"aws_vpc"},{"address":"aws_vpc.c","type":"aws_vpc"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    let edge = |from, to| crate::model::architecture::Edge {
        from,
        to,
        kind: EdgeKind::Containment,
        change: None,
    };
    assert_eq!(ContainmentTree::new(&graph).roots, vec![0, 1, 2]);
    graph.edges = vec![edge(0, 1), edge(1, 2)];
    let tree = ContainmentTree::new(&graph);
    assert_eq!(tree.roots, vec![0]);
    assert_eq!(tree.children, vec![vec![1], vec![2], vec![]]);
    assert!(tree.is_ancestor(0, 2));
    assert!(!tree.is_ancestor(2, 0));
    graph.edges.reverse();
    assert_eq!(tree, ContainmentTree::new(&graph));
    graph.edges.push(edge(0, 2));
    assert_eq!(
        ContainmentTree::new(&graph).parents,
        vec![None, Some(0), None]
    );
    graph.edges = vec![edge(0, 1), edge(1, 0), edge(1, 2)];
    assert_eq!(ContainmentTree::new(&graph).parents, vec![None, None, None]);
}
