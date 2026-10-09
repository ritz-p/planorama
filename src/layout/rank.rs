use crate::model::architecture::Graph;
use std::collections::BTreeSet;

pub(super) fn compute(graph: &Graph) -> Vec<usize> {
    let edges: Vec<_> = graph.edges.iter().map(|edge| edge.endpoints()).collect();
    compute_edges(graph.nodes.len(), &edges)
}

pub(super) fn compute_edges(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut next = vec![Vec::new(); n];
    let mut previous = vec![Vec::new(); n];
    for &(a, b) in edges {
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
    for &(a, b) in edges {
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
#[path = "../../tests/unit/layout/rank.rs"]
mod tests;
