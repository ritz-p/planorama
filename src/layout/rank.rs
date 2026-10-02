use crate::model::Graph;
use std::collections::BTreeSet;

/// Condense strongly connected components, then rank the resulting DAG.
/// Iterative DFS avoids a call-stack limit for long dependency chains.
pub(super) fn compute(graph: &Graph) -> Vec<usize> {
    let n = graph.nodes.len();
    let mut next = vec![Vec::new(); n];
    let mut previous = vec![Vec::new(); n];
    for &(a, b) in &graph.edges {
        next[a].push(b);
        previous[b].push(a);
    }
    let mut visited = vec![false; n];
    let mut order = Vec::new();
    for root in 0..n {
        if visited[root] {
            continue;
        }
        let mut stack = vec![(root, false)];
        while let Some((node, done)) = stack.pop() {
            match (done, visited[node]) {
                (true, _) => {
                    order.push(node);
                    continue;
                }
                (false, true) => continue,
                (false, false) => {}
            }
            visited[node] = true;
            stack.push((node, true));
            for &child in next[node].iter().rev() {
                if !visited[child] {
                    stack.push((child, false));
                }
            }
        }
    }
    let mut component = vec![usize::MAX; n];
    let mut count = 0;
    for &root in order.iter().rev() {
        if component[root] != usize::MAX {
            continue;
        }
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if component[node] != usize::MAX {
                continue;
            }
            component[node] = count;
            stack.extend(previous[node].iter().copied());
        }
        count += 1;
    }
    let mut links = vec![BTreeSet::new(); count];
    let mut degree = vec![0; count];
    for &(a, b) in &graph.edges {
        let (a, b) = (component[a], component[b]);
        if a != b && links[a].insert(b) {
            degree[b] += 1;
        }
    }
    let mut ready: BTreeSet<_> = (0..count).filter(|&i| degree[i] == 0).collect();
    let mut rank = vec![0; count];
    while let Some(a) = ready.pop_first() {
        for &b in &links[a] {
            rank[b] = rank[b].max(rank[a] + 1);
            degree[b] -= 1;
            if degree[b] == 0 {
                ready.insert(b);
            }
        }
    }
    component.into_iter().map(|c| rank[c]).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, Node};
    #[test]
    fn cycle_is_condensed_and_dependents_follow() {
        let nodes = (0..4)
            .map(|i| Node {
                address: i.to_string(),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
            })
            .collect();
        let graph = Graph {
            nodes,
            edges: vec![(0, 1), (1, 0), (1, 2), (2, 3)],
        };
        assert_eq!(compute(&graph), vec![0, 0, 1, 2]);
    }
}
