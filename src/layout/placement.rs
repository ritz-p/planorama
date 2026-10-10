use super::{Band, Point};
use crate::model::architecture::Graph;
use std::collections::BTreeMap;
mod expanded;
use expanded::Expanded;

#[cfg(test)]
#[path = "../../tests/unit/layout/placement.rs"]
mod tests;

const COLUMN_STEP: usize = 420;
const ROW_STEP: usize = 144;

pub(super) struct Placement<'a> {
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
    height: usize,
}

fn update_positions(
    groups: &mut [Group<'_>],
    positions: &mut [Point],
    heights: &[usize],
    column_step: usize,
) -> (usize, Vec<usize>) {
    let mut top = 160;
    let mut channels = Vec::new();
    for group in groups {
        group.top = top;
        let mut offset = 0;
        for row in 0..group.rows {
            channels.push(top + 32 + offset);
            let mut height = super::NODE_HEIGHT;
            for (column, members) in group.columns.iter().enumerate() {
                if let Some(&node) = members.get(row) {
                    positions[node] = Point {
                        x: 60 + column * column_step,
                        y: top + 48 + offset,
                    };
                    height = height.max(heights[node]);
                }
            }
            offset += height + ROW_STEP - super::NODE_HEIGHT;
        }
        channels.push(top + 32 + offset);
        group.height = offset + 56;
        top += offset + 80;
    }
    (top, channels)
}
pub(super) fn place<'a>(graph: &'a Graph, ranks: &[usize]) -> Placement<'a> {
    place_with_heights(graph, ranks, &vec![super::NODE_HEIGHT; graph.nodes.len()])
}

pub(super) fn place_with_heights<'a>(
    graph: &'a Graph,
    ranks: &[usize],
    heights: &[usize],
) -> Placement<'a> {
    let expanded = Expanded::new(graph, ranks);
    let heights: Vec<_> = expanded
        .vertices
        .iter()
        .map(|vertex| {
            vertex
                .resource
                .map_or(super::NODE_HEIGHT, |node| heights[node])
        })
        .collect();
    let column_step =
        COLUMN_STEP.max(super::NODE_WIDTH + super::relationship_markers::padding(graph));
    let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
    let mut modules = BTreeMap::new();
    for (node, vertex) in expanded.vertices.iter().enumerate() {
        modules
            .entry(vertex.module)
            .or_insert_with(|| vec![Vec::new(); columns])[vertex.rank]
            .push(node);
    }
    let mut groups: Vec<_> = modules
        .into_iter()
        .map(|(label, columns)| Group {
            rows: columns.iter().map(Vec::len).max().unwrap_or(0),
            label,
            columns,
            top: 0,
            height: 0,
        })
        .collect();
    groups.sort_by_key(|g| (g.label != "root", g.label));
    let mut positions = vec![Point { x: 0, y: 0 }; expanded.vertices.len()];
    update_positions(&mut groups, &mut positions, &heights, column_step);
    for sweep in 0..6 {
        let forward = sweep % 2 == 0;
        for step in 0..columns {
            let (rank, neighbors) = match forward {
                true => (step, &expanded.incoming),
                false => (columns - step - 1, &expanded.outgoing),
            };
            for group in &mut groups {
                group.columns[rank].sort_by_cached_key(|&node| {
                    let ys: Vec<_> = neighbors[node]
                        .iter()
                        .copied()
                        .filter(|&other| match forward {
                            true => expanded.vertices[other].rank < rank,
                            false => expanded.vertices[other].rank > rank,
                        })
                        .map(|other| positions[other].y as u64)
                        .collect();
                    match ys.as_slice() {
                        [] => positions[node].y as u64 * 1024,
                        ys => ys.iter().sum::<u64>() * 1024 / ys.len() as u64,
                    }
                });
            }
            update_positions(&mut groups, &mut positions, &heights, column_step);
        }
    }

    let (top, channels) = update_positions(&mut groups, &mut positions, &heights, column_step);
    let mut resource_positions = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
    for (vertex, position) in expanded.vertices.iter().zip(positions) {
        if let Some(resource) = vertex.resource {
            resource_positions[resource] = position;
        }
    }
    Placement {
        height: (top + 40).max(300),
        positions: resource_positions,
        channels,
        bands: groups
            .into_iter()
            .map(|g| Band {
                label: g.label,
                top: g.top,
                height: g.height,
            })
            .collect(),
    }
}
