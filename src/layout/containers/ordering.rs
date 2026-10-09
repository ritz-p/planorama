use crate::model::{Graph, ResourceRole};
use std::collections::{BTreeSet, VecDeque};

pub(in crate::layout) fn structural_keys(graph: &Graph) -> Vec<usize> {
    fn ranks<T: Ord>(keys: &[T]) -> Vec<usize> {
        let unique: BTreeSet<_> = keys.iter().collect();
        let ordered: Vec<_> = unique.into_iter().collect();
        keys.iter()
            .map(|key| ordered.binary_search(&key).unwrap())
            .collect()
    }
    let labels: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.resource_address().local(),
                node.resource_type.as_str(),
                format!("{:?}/{:?}/{:?}", node.role, node.mode, node.action),
            )
        })
        .collect();
    let mut colors = ranks(&labels);
    let mut neighbors = vec![Vec::new(); graph.nodes.len()];
    for edge in &graph.edges {
        let action = edge.change.as_ref().map(|change| change.action);
        let identity = edge.change.as_ref().map(|change| change.local_address());
        neighbors[edge.from].push((true, edge.kind, action, identity, edge.to));
        neighbors[edge.to].push((false, edge.kind, action, identity, edge.from));
    }
    loop {
        let signatures: Vec<_> = neighbors
            .iter()
            .enumerate()
            .map(|(node, edges)| {
                let mut adjacent: Vec<_> = edges
                    .iter()
                    .map(|&(outgoing, kind, action, identity, other)| {
                        (outgoing, kind, action, identity, colors[other])
                    })
                    .collect();
                adjacent.sort();
                (colors[node], adjacent)
            })
            .collect();
        let next = ranks(&signatures);
        match next == colors {
            true => return colors,
            false => colors = next,
        }
    }
}

pub(in crate::layout) fn roots(
    graph: &Graph,
    parents: &[Option<usize>],
    keys: &[usize],
) -> Vec<usize> {
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
            keys[node],
            graph.nodes[node].entity.id.as_str(),
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
