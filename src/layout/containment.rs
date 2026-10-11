use super::containers::ordering;
use crate::model::architecture::{EdgeKind, Graph, ResourceRole};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainmentTree {
    pub parents: Vec<Option<usize>>,
    pub children: Vec<Vec<usize>>,
    pub roots: Vec<usize>,
    pub(super) keys: Vec<usize>,
}

impl ContainmentTree {
    pub fn routing_peers(
        &self,
        from: usize,
        to: usize,
        bounds: &[super::Bounds],
    ) -> Vec<super::Bounds> {
        if self.parents[from] != self.parents[to]
            || self.is_ancestor(from, to)
            || self.is_ancestor(to, from)
        {
            return Vec::new();
        }
        let a = bounds[from];
        let b = bounds[to];
        let y1 = a.origin.y + a.height / 2;
        let y2 = b.origin.y + b.height / 2;
        bounds
            .iter()
            .enumerate()
            .filter_map(|(node, &obstacle)| {
                (node != from
                    && node != to
                    && self.parents[node] == self.parents[from]
                    && obstacle.origin.x >= a.right().min(b.right())
                    && obstacle.right() <= a.origin.x.max(b.origin.x)
                    && obstacle.origin.y < y1.min(y2)
                    && obstacle.origin.y + obstacle.height > y1.max(y2))
                .then_some(obstacle)
            })
            .collect()
    }

    pub fn new(graph: &Graph) -> Self {
        let parents = parents(graph);
        let keys = ordering::structural_keys(graph);
        let mut children = vec![Vec::new(); graph.nodes.len()];
        for (node, parent) in parents.iter().enumerate() {
            if let Some(parent) = parent {
                children[*parent].push(node);
            }
        }
        for nodes in &mut children {
            nodes.sort_by_key(|&node| {
                (
                    graph.nodes[node].resource_address().local(),
                    keys[node],
                    graph.nodes[node].entity.id.as_str(),
                )
            });
        }
        let roots = ordering::roots(graph, &parents, &keys);
        Self {
            parents,
            children,
            roots,
            keys,
        }
    }
    pub fn is_ancestor(&self, ancestor: usize, mut node: usize) -> bool {
        loop {
            if ancestor == node {
                return true;
            }
            match self.parents[node] {
                Some(parent) => node = parent,
                None => return false,
            }
        }
    }
}

fn parents(graph: &Graph) -> Vec<Option<usize>> {
    let mut candidates = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Containment
            && edge.from != edge.to
            && graph.nodes[edge.from].role == ResourceRole::Container
        {
            candidates[edge.to].insert(edge.from);
        }
    }
    let mut parents: Vec<_> = candidates
        .iter()
        .map(|values| match values.len() {
            1 => values.first().copied(),
            _ => None,
        })
        .collect();
    let original = parents.clone();
    for (node, parent) in parents.iter_mut().enumerate() {
        let mut visited = BTreeSet::from([node]);
        let mut current = original[node];
        while let Some(ancestor) = current {
            if !visited.insert(ancestor) {
                *parent = None;
                break;
            }
            current = original[ancestor];
        }
    }
    parents
}

#[cfg(test)]
#[path = "../../tests/unit/layout/containment.rs"]
mod tests;
