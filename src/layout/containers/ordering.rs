use crate::model::{Graph, ResourceRole};
use std::collections::{BTreeSet, VecDeque};

pub(super) fn roots(graph: &Graph, parents: &[Option<usize>]) -> Vec<usize> {
    let owners: Vec<_> = (0..graph.nodes.len())
        .map(|mut node| {
            while let Some(parent) = parents[node] {
                node = parent;
            }
            node
        })
        .collect();
    let mut neighbors = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        let (from, to) = (owners[edge.from], owners[edge.to]);
        if from != to {
            neighbors[from].insert(to);
            neighbors[to].insert(from);
        }
    }
    let key = |node: usize| {
        (
            graph.nodes[node].role != ResourceRole::Container,
            graph.nodes[node].resource_address().local(),
            graph.nodes[node].address.as_str(),
        )
    };
    let mut roots: Vec<_> = parents
        .iter()
        .enumerate()
        .filter_map(|(node, parent)| parent.is_none().then_some(node))
        .collect();
    roots.sort_by_key(|&node| key(node));
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    for root in roots {
        if !visited.insert(root) {
            continue;
        }
        let mut pending = VecDeque::from([root]);
        while let Some(node) = pending.pop_front() {
            ordered.push(node);
            let mut adjacent: Vec<_> = neighbors[node].iter().copied().collect();
            adjacent.sort_by_key(|&node| key(node));
            for neighbor in adjacent {
                if visited.insert(neighbor) {
                    pending.push_back(neighbor);
                }
            }
        }
    }
    ordered
}
