use crate::layout::rank;
use crate::model::{EdgeKind, Graph};
use std::collections::{BTreeMap, BTreeSet};

/// Project relationships between descendant resources onto the direct children.
/// Nesting controls membership; only non-containment edges determine columns.
pub(super) fn columns(
    graph: &Graph,
    parent: usize,
    children: &[usize],
    parents: &[Option<usize>],
) -> Vec<Vec<usize>> {
    let indices: BTreeMap<_, _> = children
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();
    let owners: Vec<_> = (0..graph.nodes.len())
        .map(|mut node| {
            while let Some(ancestor) = parents[node] {
                if ancestor == parent {
                    return indices.get(&node).copied();
                }
                node = ancestor;
            }
            None
        })
        .collect();
    let edges: BTreeSet<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind != EdgeKind::Containment)
        .filter_map(|edge| {
            let (from, to) = (owners[edge.from]?, owners[edge.to]?);
            (from != to).then_some((from, to))
        })
        .collect();
    let ranks = rank::compute_edges(children.len(), &edges.into_iter().collect::<Vec<_>>());
    let mut columns = vec![Vec::new(); ranks.iter().max().map_or(0, |rank| rank + 1)];
    // Keep the structural ordering computed by the caller within each rank.
    for (&node, rank) in children.iter().zip(ranks) {
        columns[rank].push(node);
    }
    columns
}
