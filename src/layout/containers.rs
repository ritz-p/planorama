use super::{Band, Bounds, Layout, NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::{EdgeKind, Graph, ResourceRole};
use std::collections::{BTreeMap, BTreeSet};
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
    let mut children = vec![Vec::new(); graph.nodes.len()];
    let mut modules: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (node, parent) in parents.iter().enumerate() {
        match parent {
            Some(parent) => children[*parent].push(node),
            None => modules
                .entry(&graph.nodes[node].module)
                .or_default()
                .push(node),
        }
    }
    for nodes in children.iter_mut().chain(modules.values_mut()) {
        nodes.sort_by_key(|&node| &graph.nodes[node].address);
    }
    let mut order = Vec::new();
    let mut pending: Vec<_> = modules.values().flatten().copied().collect();
    while let Some(node) = pending.pop() {
        order.push(node);
        pending.extend(children[node].iter().copied());
    }
    let mut sizes = vec![(NODE_WIDTH, NODE_HEIGHT); graph.nodes.len()];
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
            sizes[node] = (width + PADDING * 2, NODE_HEIGHT + PADDING + height);
        }
    }
    let mut layout = Layout {
        width: 1100,
        height: 300,
        positions: vec![Point { x: 0, y: 0 }; graph.nodes.len()],
        bands: Vec::new(),
        paths: Vec::new(),
        junctions: Vec::new(),
        containers: Vec::new(),
        parents,
    };
    let mut modules: Vec<_> = modules.into_iter().collect();
    modules.sort_by_key(|(name, _)| (*name != "root", *name));
    let mut top = 160;
    for (label, roots) in modules {
        let mut y = top + 48;
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
                let mut child_y = origin.y + NODE_HEIGHT + PADDING;
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
        layout.bands.push(Band {
            label,
            top,
            height: y - top,
        });
        top = y + PADDING;
    }
    layout.height = top + 40;
    layout.paths = routing::route(graph, &layout);
    layout
}
