use super::{PADDING, Point};
use crate::model::architecture::{EdgeKind, Graph, ResourceRole};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Group {
    pub target: usize,
    pub sources: Vec<usize>,
    pub edges: Vec<usize>,
}

pub(super) fn groups(graph: &Graph, parents: &[Option<usize>]) -> Vec<Group> {
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
                candidates.push((offsets[other].y + sizes[other].1 + PADDING).min(max_y));
                candidates.push(offsets[other].y.saturating_sub(sizes[target].1 + PADDING));
            }
        }
        let x = offsets[target].x;
        if let Some(y) = candidates
            .into_iter()
            .filter(|&y| {
                y <= max_y
                    && children.iter().all(|&other| {
                        other == target
                            || x + sizes[target].0 + PADDING <= offsets[other].x
                            || offsets[other].x + sizes[other].0 + PADDING <= x
                            || y + sizes[target].1 + PADDING <= offsets[other].y
                            || offsets[other].y + sizes[other].1 + PADDING <= y
                    })
            })
            .min_by_key(|&y| (y.abs_diff(desired), y))
        {
            offsets[target].y = y;
        }
    }
}
