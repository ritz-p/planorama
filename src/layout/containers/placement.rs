use crate::layout::Point;
use crate::layout::rank;
use crate::model::architecture::{EdgeKind, Graph};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::layout) fn relationships(
    graph: &Graph,
    children: &[usize],
    parents: &[Option<usize>],
) -> Vec<(usize, usize)> {
    let members: BTreeSet<_> = children.iter().copied().collect();
    let owners: Vec<_> = (0..graph.nodes.len())
        .map(|mut node| {
            loop {
                if members.contains(&node) {
                    return Some(node);
                }
                match parents[node] {
                    Some(parent) => node = parent,
                    None => return None,
                }
            }
        })
        .collect();
    graph
        .edges
        .iter()
        .filter(|edge| edge.kind != EdgeKind::Containment)
        .filter_map(|edge| {
            let (a, b) = (owners[edge.from]?, owners[edge.to]?);
            (a != b).then_some((a.min(b), a.max(b)))
        })
        .collect()
}

pub(in crate::layout) fn refine(
    children: &[usize],
    relationships: &[(usize, usize)],
    sizes: &[(usize, usize)],
    offsets: &mut [Point],
    height: usize,
    padding: usize,
) {
    let mut columns: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &node in children {
        columns.entry(offsets[node].x).or_default().push(node);
    }
    for column in columns.values_mut() {
        column.sort_by_key(|&node| offsets[node].y);
    }
    let mut neighbors = vec![Vec::new(); sizes.len()];
    for &(a, b) in relationships {
        neighbors[a].push(b);
        neighbors[b].push(a);
    }
    let center = |node: usize, positions: &[Point]| positions[node].y * 2 + sizes[node].1;
    let cost = |positions: &[Point]| -> u128 {
        relationships
            .iter()
            .map(|&(a, b)| center(a, positions).abs_diff(center(b, positions)) as u128)
            .sum()
    };
    let mut best_cost = cost(offsets);
    for sweep in 0..8 {
        let mut keys: Vec<_> = columns.keys().copied().collect();
        if sweep % 2 != 0 {
            keys.reverse();
        }
        let mut improved = false;
        for x in keys {
            let column = &columns[&x];
            let mut ordered = column.clone();
            let preferred = |node: usize| -> (u128, u128) {
                if neighbors[node].is_empty() {
                    (center(node, offsets) as u128, 1)
                } else {
                    (
                        neighbors[node]
                            .iter()
                            .map(|&peer| center(peer, offsets) as u128)
                            .sum(),
                        neighbors[node].len() as u128,
                    )
                }
            };
            ordered.sort_by(|&a, &b| {
                let (a_sum, a_count) = preferred(a);
                let (b_sum, b_count) = preferred(b);
                (a_sum * b_count).cmp(&(b_sum * a_count))
            });
            let original: Vec<_> = column.iter().map(|&node| (node, offsets[node])).collect();
            let mut y = 0;
            for &node in &ordered {
                offsets[node].y = y;
                y += sizes[node].1 + padding;
            }
            let packed_height = y.saturating_sub(padding);
            let mut shifts = Vec::new();
            for &node in &ordered {
                for &peer in &neighbors[node] {
                    if offsets[peer].x != x {
                        shifts.push(
                            (center(peer, offsets).saturating_sub(center(node, offsets)) / 2)
                                .min(height.saturating_sub(packed_height)),
                        );
                    }
                }
            }
            shifts.sort_unstable();
            let shift = shifts
                .get(shifts.len().saturating_sub(1) / 2)
                .copied()
                .unwrap_or(0);
            for &node in &ordered {
                offsets[node].y += shift;
            }
            let next_cost = cost(offsets);
            if next_cost < best_cost {
                best_cost = next_cost;
                columns.insert(x, ordered);
                improved = true;
            } else {
                for (node, position) in original {
                    offsets[node] = position;
                }
            }
        }
        if !improved && sweep % 2 != 0 {
            break;
        }
    }
}

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
    padding: usize,
) -> (usize, usize) {
    let nodes: Vec<_> = columns.iter().flatten().copied().collect();
    if nodes.is_empty() {
        return (0, 0);
    }
    let minimum = nodes.iter().map(|&node| sizes[node].1).max().unwrap();
    let total: usize = nodes.iter().map(|&node| sizes[node].1 + padding).sum();
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
                    x += width + padding;
                    y = 0;
                    width = 0;
                }
                positions.push((node, Point { x, y }));
                height = height.max(y + h);
                width = width.max(w);
                y += h + padding;
            }
            x += width + padding;
        }
        let width = x.saturating_sub(padding);
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
