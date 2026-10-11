use super::Point;
use crate::model::architecture::{EdgeKind, Graph, ResourceRole};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::layout) struct Group {
    pub reverse: bool,
    pub target: usize,
    pub sources: Vec<usize>,
    pub edges: Vec<usize>,
}

pub(super) fn movable(children: &[usize], groups: &[Group], offsets: &[Point]) -> Vec<usize> {
    let columns: BTreeSet<_> = groups
        .iter()
        .filter(|group| children.contains(&group.target))
        .flat_map(|group| group.sources.iter().chain(std::iter::once(&group.target)))
        .map(|&node| offsets[node].x)
        .collect();
    children
        .iter()
        .copied()
        .filter(|&node| !columns.contains(&offsets[node].x))
        .collect()
}

pub(super) fn deviation(
    children: &[usize],
    groups: &[Group],
    sizes: &[(usize, usize)],
    offsets: &[Point],
) -> u128 {
    groups
        .iter()
        .filter(|group| {
            children.contains(&group.target)
                && group.sources.iter().all(|source| children.contains(source))
        })
        .map(|group| {
            let mut centers: Vec<_> = group
                .sources
                .iter()
                .map(|&node| offsets[node].y + sizes[node].1 / 2)
                .collect();
            centers.sort_unstable();
            let median = (centers[(centers.len() - 1) / 2] + centers[centers.len() / 2]) / 2;
            (offsets[group.target].y + sizes[group.target].1 / 2).abs_diff(median) as u128
        })
        .sum()
}

pub(in crate::layout) fn protected_roots(
    graph: &Graph,
    parents: &[Option<usize>],
) -> BTreeSet<usize> {
    let owners: Vec<_> = (0..graph.nodes.len())
        .map(|mut node| {
            while let Some(parent) = parents[node] {
                node = parent;
            }
            node
        })
        .collect();
    let groups = groups(graph, parents);
    let members: BTreeSet<_> = groups
        .iter()
        .flat_map(|group| {
            group
                .sources
                .iter()
                .copied()
                .chain(std::iter::once(group.target))
        })
        .collect();
    let mut protected: BTreeSet<_> = members.iter().map(|&node| owners[node]).collect();
    for edge in &graph.edges {
        if members.contains(&edge.from) || members.contains(&edge.to) {
            protected.insert(owners[edge.from]);
            protected.insert(owners[edge.to]);
        }
    }
    protected
}

pub(in crate::layout) fn groups(graph: &Graph, parents: &[Option<usize>]) -> Vec<Group> {
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.kind == EdgeKind::Connection
            && edge.change.is_none()
            && graph.nodes[edge.from].role == ResourceRole::Container
            && graph.nodes[edge.to].role != ResourceRole::Container
            && parents[edge.to].is_some()
            && parents[edge.from] == parents[edge.to]
        {
            groups.entry(edge.to).or_default().push(index);
        }
    }
    groups
        .into_iter()
        .filter_map(|(target, edges)| {
            let sources: BTreeSet<_> = edges.iter().map(|&i| graph.edges[i].from).collect();
            (sources.len() > 1 && sources.len() == edges.len()).then(|| Group {
                reverse: false,
                target,
                sources: sources.into_iter().collect(),
                edges,
            })
        })
        .collect()
}

pub(super) fn cohere(columns: &mut [Vec<usize>], groups: &[Group]) {
    for column in columns {
        let mut representatives: Vec<_> = (0..column.len()).collect();
        for group in groups {
            let members: Vec<_> = column
                .iter()
                .enumerate()
                .filter(|(_, node)| group.sources.contains(node))
                .map(|(i, _)| i)
                .collect();
            if let Some(root) = members.iter().map(|&i| representatives[i]).min() {
                let connected: BTreeSet<_> = members.iter().map(|&i| representatives[i]).collect();
                for representative in &mut representatives {
                    if connected.contains(representative) {
                        *representative = root;
                    }
                }
            }
        }
        let mut order: Vec<_> = column.iter().copied().enumerate().collect();
        order.sort_by_key(|&(i, _)| (representatives[i], i));
        *column = order.into_iter().map(|(_, node)| node).collect();
    }
}

pub(super) fn align(
    children: &[usize],
    groups: &[Group],
    sizes: &[(usize, usize)],
    offsets: &mut [Point],
    height: usize,
    padding: usize,
) {
    for &target in children {
        let Some(group) = groups.iter().find(|group| group.target == target) else {
            continue;
        };
        if !group.sources.iter().all(|source| children.contains(source)) {
            continue;
        }
        let mut centers: Vec<_> = group
            .sources
            .iter()
            .map(|&node| offsets[node].y + sizes[node].1 / 2)
            .collect();
        centers.sort_unstable();
        let center = (centers[(centers.len() - 1) / 2] + centers[centers.len() / 2]) / 2;
        let desired = center.saturating_sub(sizes[target].1 / 2);
        let max_y = height.saturating_sub(sizes[target].1);
        let mut candidates = vec![offsets[target].y, desired.min(max_y)];
        for &other in children {
            if other != target {
                candidates.push((offsets[other].y + sizes[other].1 + padding).min(max_y));
                candidates.push(offsets[other].y.saturating_sub(sizes[target].1 + padding));
            }
        }
        let x = offsets[target].x;
        if let Some(y) = candidates
            .into_iter()
            .filter(|&y| {
                y <= max_y
                    && children.iter().all(|&other| {
                        other == target
                            || x + sizes[target].0 + padding <= offsets[other].x
                            || offsets[other].x + sizes[other].0 + padding <= x
                            || y + sizes[target].1 + padding <= offsets[other].y
                            || offsets[other].y + sizes[other].1 + padding <= y
                    })
            })
            .min_by_key(|&y| (y.abs_diff(desired), y))
        {
            offsets[target].y = y;
        }
    }
}
