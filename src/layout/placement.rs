//! Place resource cards in module bands and reduce edge crossings.
use super::{Band, Point};
use crate::model::Graph;
use std::collections::BTreeMap;

const COLUMN_STEP: usize = 420;
const ROW_STEP: usize = 144;

pub(super) struct Placement<'a> {
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    pub channels: Vec<usize>,
}

struct Group<'a> {
    label: &'a str,
    columns: Vec<Vec<usize>>,
    top: usize,
    rows: usize,
}

fn update_positions(groups: &[Group<'_>], positions: &mut [Point]) {
    for group in groups {
        for (column, members) in group.columns.iter().enumerate() {
            for (row, &node) in members.iter().enumerate() {
                positions[node] = Point {
                    x: 60 + column * COLUMN_STEP,
                    y: group.top + 48 + row * ROW_STEP,
                };
            }
        }
    }
}

pub(super) fn place<'a>(graph: &'a Graph, ranks: &[usize]) -> Placement<'a> {
    let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
    let mut modules = BTreeMap::new();
    for (node, resource) in graph.nodes.iter().enumerate() {
        modules
            .entry(resource.module.as_str())
            .or_insert_with(|| vec![Vec::new(); columns])[ranks[node]]
            .push(node);
    }
    let mut groups: Vec<_> = modules
        .into_iter()
        .map(|(label, columns)| Group {
            rows: columns.iter().map(Vec::len).max().unwrap_or(0),
            label,
            columns,
            top: 0,
        })
        .collect();
    // Keep root first, then module paths in deterministic lexical order.
    groups.sort_by_key(|g| (g.label != "root", g.label));
    let mut top = 160;
    let mut channels = Vec::new();
    for group in &mut groups {
        group.top = top;
        for row in 0..=group.rows {
            channels.push(top + 32 + row * ROW_STEP);
        }
        top += group.rows * ROW_STEP + 80;
    }
    let mut positions = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
    update_positions(&groups, &mut positions);
    let mut incoming = vec![Vec::new(); graph.nodes.len()];
    let mut outgoing = vec![Vec::new(); graph.nodes.len()];
    for &(a, b) in &graph.edges {
        incoming[b].push(a);
        outgoing[a].push(b);
    }
    // Alternating barycentric sweeps reduce crossings without disturbing
    // module bands or dependency ranks. Ties retain their previous order.
    for sweep in 0..6 {
        let forward = sweep % 2 == 0;
        for step in 0..columns {
            let (rank, neighbors) = match forward {
                true => (step, &incoming),
                false => (columns - step - 1, &outgoing),
            };
            for group in &mut groups {
                group.columns[rank].sort_by_cached_key(|&node| {
                    let ys: Vec<_> = neighbors[node]
                        .iter()
                        .copied()
                        .filter(|&other| match forward {
                            true => ranks[other] < rank,
                            false => ranks[other] > rank,
                        })
                        .map(|other| positions[other].y as u64)
                        .collect();
                    match ys.as_slice() {
                        [] => positions[node].y as u64 * 1024,
                        ys => ys.iter().sum::<u64>() * 1024 / ys.len() as u64,
                    }
                });
            }
            update_positions(&groups, &mut positions);
        }
    }

    Placement {
        width: (columns * COLUMN_STEP + 80).max(1040),
        height: (top + 40).max(300),
        positions,
        channels,
        bands: groups
            .into_iter()
            .map(|g| Band {
                label: g.label,
                top: g.top,
                height: g.rows * ROW_STEP + 56,
            })
            .collect(),
    }
}
