use super::{Bounds, Layout, NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::{EdgeKind, Graph, ResourceRole};
use std::collections::BTreeSet;
mod ordering;
mod routing;

#[cfg(test)]
#[path = "../../tests/unit/layout/containers.rs"]
mod tests;

const PADDING: usize = 40;

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

pub(super) fn place(graph: &Graph) -> Layout<'_> {
    let parents = parents(graph);
    let header_heights: Vec<_> = routing::incidents(graph, &parents)
        .iter()
        .map(|edges| NODE_HEIGHT.max(edges.len() + 41))
        .collect();
    let mut children = vec![Vec::new(); graph.nodes.len()];
    let keys = ordering::structural_keys(graph);
    for (node, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            children[*parent].push(node);
        }
    }
    for nodes in &mut children {
        nodes.sort_by_key(|&node| (graph.nodes[node].resource_address().local(), keys[node]));
    }
    let roots = ordering::roots(graph, &parents, &keys);
    let mut order = Vec::new();
    let mut pending = roots.clone();
    while let Some(node) = pending.pop() {
        order.push(node);
        pending.extend(children[node].iter().copied());
    }
    let mut sizes: Vec<_> = header_heights
        .iter()
        .map(|&height| (NODE_WIDTH, height))
        .collect();
    for &node in order.iter().rev() {
        if graph.nodes[node].role == ResourceRole::Container {
            let width = children[node]
                .iter()
                .map(|&child| sizes[child].0)
                .max()
                .unwrap_or(NODE_WIDTH);
            let height: usize = children[node]
                .iter()
                .map(|&child| sizes[child].1 + PADDING)
                .sum();
            sizes[node] = (width + PADDING * 2, header_heights[node] + PADDING + height);
        }
    }
    let mut layout = Layout {
        header_heights,
        width: 1100,
        height: 300,
        positions: vec![Point { x: 0, y: 0 }; graph.nodes.len()],
        bands: Vec::new(),
        paths: Vec::new(),
        junctions: Vec::new(),
        containers: Vec::new(),
        parents,
    };
    let mut y = 160;
    for root in roots {
        layout.positions[root] = Point { x: 60, y };
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            let origin = layout.positions[node];
            let (width, height) = sizes[node];
            layout.width = layout.width.max(origin.x + width + 80);
            if graph.nodes[node].role == ResourceRole::Container {
                layout.containers.push((
                    node,
                    Bounds {
                        origin,
                        width,
                        height,
                    },
                ));
            }
            let mut child_y = origin.y + layout.header_heights[node] + PADDING;
            for &child in &children[node] {
                layout.positions[child] = Point {
                    x: origin.x + PADDING,
                    y: child_y,
                };
                child_y += sizes[child].1 + PADDING;
            }
            pending.extend(children[node].iter().rev().copied());
        }
        y += sizes[root].1 + PADDING;
    }
    layout.height = (y + 40).max(300);
    layout.paths = routing::route(graph, &layout);
    layout
}
