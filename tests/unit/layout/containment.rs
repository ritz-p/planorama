use super::*;

#[test]
fn hierarchy_preserves_roots_nested_children_ambiguity_and_cycles() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"aws_vpc.a","type":"aws_vpc"},{"address":"aws_vpc.b","type":"aws_vpc"},{"address":"aws_vpc.c","type":"aws_vpc"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    let edge = |from, to, kind| crate::model::architecture::Edge {
        from,
        to,
        kind,
        change: None,
    };
    let mut check =
        |edges, parents: [Option<usize>; 3], roots: &[usize], children: [Vec<usize>; 3]| {
            graph.edges = edges;
            let tree = ContainmentTree::new(&graph);
            assert_eq!(tree.parents, parents);
            assert_eq!(tree.roots, roots);
            assert_eq!(tree.children, children);
            graph.edges.reverse();
            assert_eq!(ContainmentTree::new(&graph), tree);
            tree
        };
    use EdgeKind::{Containment, Dependency};
    check(vec![], [None; 3], &[0, 1, 2], [vec![], vec![], vec![]]);
    check(
        vec![edge(0, 1, Containment), edge(1, 2, Dependency)],
        [None, Some(0), None],
        &[0, 2],
        [vec![1], vec![], vec![]],
    );
    let nested = check(
        vec![edge(0, 1, Containment), edge(1, 2, Containment)],
        [None, Some(0), Some(1)],
        &[0],
        [vec![1], vec![2], vec![]],
    );
    assert!(nested.is_ancestor(0, 2));
    assert!(!nested.is_ancestor(2, 0));
    check(
        vec![
            edge(0, 1, Containment),
            edge(1, 2, Containment),
            edge(0, 2, Containment),
        ],
        [None, Some(0), None],
        &[0, 2],
        [vec![1], vec![], vec![]],
    );
    check(
        vec![
            edge(0, 1, Containment),
            edge(1, 0, Containment),
            edge(1, 2, Containment),
        ],
        [None; 3],
        &[0, 1, 2],
        [vec![], vec![], vec![]],
    );
}
