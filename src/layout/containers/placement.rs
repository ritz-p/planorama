use super::PADDING;
use crate::layout::Point;
use crate::layout::rank;
use crate::model::architecture::{EdgeKind, Graph};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::layout) fn columns(
    graph: &Graph,
    parent: Option<usize>,
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
            loop {
                if parents[node] == parent {
                    return indices.get(&node).copied();
                }
                match parents[node] {
                    Some(ancestor) => node = ancestor,
                    None => return None,
                }
            }
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
    for (&node, rank) in children.iter().zip(ranks) {
        columns[rank].push(node);
    }
    columns
}

pub(in crate::layout) fn pack(
    columns: &[Vec<usize>],
    sizes: &[(usize, usize)],
    offsets: &mut [Point],
) -> (usize, usize) {
    let nodes: Vec<_> = columns.iter().flatten().copied().collect();
    if nodes.is_empty() {
        return (0, 0);
    }
    let minimum = nodes.iter().map(|&node| sizes[node].1).max().unwrap();
    let total: usize = nodes.iter().map(|&node| sizes[node].1 + PADDING).sum();
    let heights: BTreeSet<_> = (1..=nodes.len().min(64))
        .map(|count| (total / count).max(minimum))
        .chain([minimum])
        .collect();
    let mut best = None;
    for limit in heights {
        let (mut x, mut height) = (0, 0);
        let mut positions = Vec::with_capacity(nodes.len());
        for column in columns {
            let (mut y, mut width) = (0, 0);
            for &node in column {
                let (w, h) = sizes[node];
                if y > 0 && y + h > limit {
                    x += width + PADDING;
                    y = 0;
                    width = 0;
                }
                positions.push((node, Point { x, y }));
                height = height.max(y + h);
                width = width.max(w);
                y += h + PADDING;
            }
            x += width + PADDING;
        }
        let width = x.saturating_sub(PADDING);
        let score = (
            (width as u128 * 10).max(height as u128 * 17),
            width as u128 * height as u128,
            width,
        );
        if best.as_ref().is_none_or(|(old, _, _)| score < *old) {
            best = Some((score, (width, height), positions));
        }
    }
    let (_, size, positions) = best.unwrap();
    for (node, position) in positions {
        offsets[node] = position;
    }
    size
}
