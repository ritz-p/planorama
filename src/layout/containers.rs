use super::{Bounds, Layout, NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::{Graph, ResourceRole};

mod affinity;
pub(super) mod ordering;
mod placement;
mod routing;

#[cfg(test)]
#[path = "../../tests/unit/layout/containers.rs"]
mod tests;

const PADDING: usize = 40;

pub(super) fn route(graph: &Graph, layout: &mut Layout<'_>, bundle: bool) {
    let routed = routing::route(graph, layout, &layout.containment.keys, bundle);
    layout.paths = routed.paths;
    layout.junctions = routed.junctions;
}

#[cfg(test)]
fn place_with_affinity(graph: &Graph, enabled: bool) -> Layout<'_> {
    let mut layout = place_geometry(graph, super::ContainmentTree::new(graph), enabled);
    route(graph, &mut layout, enabled);
    layout
}

pub(super) fn place_geometry(
    graph: &Graph,
    tree: super::ContainmentTree,
    enabled: bool,
) -> Layout<'_> {
    let parents = &tree.parents;
    let affinities = if enabled {
        affinity::groups(graph, parents)
    } else {
        Vec::new()
    };
    let header_heights: Vec<_> = routing::incidents(graph, &tree)
        .iter()
        .map(|edges| NODE_HEIGHT.max(edges.len() + 41))
        .collect();
    let children = &tree.children;

    let roots = &tree.roots;
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
    let mut offsets = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
    for &node in order.iter().rev() {
        if graph.nodes[node].role == ResourceRole::Container {
            let mut columns = placement::columns(graph, Some(node), &children[node], parents);
            affinity::cohere(&mut columns, &affinities);
            let (width, height) = placement::pack(&columns, &sizes, &mut offsets);
            affinity::align(&children[node], &affinities, &sizes, &mut offsets, height);
            for &child in &children[node] {
                offsets[child].x += PADDING;
                offsets[child].y += header_heights[node] + PADDING;
            }
            sizes[node] = (
                width.max(NODE_WIDTH) + PADDING * 2,
                header_heights[node]
                    + PADDING
                    + height
                    + if children[node].is_empty() {
                        0
                    } else {
                        PADDING
                    },
            );
        }
    }
    let columns = placement::columns(graph, None, roots, parents);
    let (_, root_height) = placement::pack(&columns, &sizes, &mut offsets);
    let mut layout = Layout {
        bounds: vec![Bounds::card(Point { x: 0, y: 0 }); graph.nodes.len()],
        header_heights,
        width: 1100,
        height: 300,
        positions: vec![Point { x: 0, y: 0 }; graph.nodes.len()],
        bands: Vec::new(),
        paths: Vec::new(),
        junctions: Vec::new(),
        containers: Vec::new(),
        containment: tree,
    };
    for &root in &layout.containment.roots {
        layout.positions[root] = Point {
            x: 60 + offsets[root].x,
            y: 160 + offsets[root].y,
        };
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            let origin = layout.positions[node];
            let (width, height) = sizes[node];
            layout.bounds[node] = Bounds {
                origin,
                width,
                height,
            };
            layout.width = layout.width.max(origin.x + width + 80);
            if graph.nodes[node].role == ResourceRole::Container {
                layout.containers.push(node);
            }
            for &child in &layout.containment.children[node] {
                layout.positions[child] = Point {
                    x: origin.x + offsets[child].x,
                    y: origin.y + offsets[child].y,
                };
            }
            pending.extend(layout.containment.children[node].iter().rev().copied());
        }
    }
    layout.height = (root_height + 240).max(300);
    layout
}
