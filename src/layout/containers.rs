use super::{Bounds, Layout, NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::architecture::{Graph, ResourceRole};

pub(super) mod affinity;
pub(super) mod ordering;
pub(super) mod placement;
pub(super) mod routing;

#[cfg(test)]
#[path = "../../tests/unit/layout/containers.rs"]
mod tests;

#[cfg(test)]
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
    let heights = header_heights(graph, &tree);
    place_with_heights(graph, tree, enabled, heights)
}

pub(super) fn place_with_heights(
    graph: &Graph,
    tree: super::ContainmentTree,
    enabled: bool,
    header_heights: Vec<usize>,
) -> Layout<'_> {
    let parents = &tree.parents;
    let padding = super::relationship_markers::padding(graph);
    let special = affinity::groups(graph, parents);
    let affinities = if enabled { special.as_slice() } else { &[] };
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
            affinity::cohere(&mut columns, affinities);
            let relationships = placement::relationships(graph, &children[node], parents);
            let preserve_affinity = special
                .iter()
                .any(|group| children[node].contains(&group.target));
            let (width, height) = placement::pack(
                &columns,
                if preserve_affinity {
                    &[]
                } else {
                    &relationships
                },
                &sizes,
                &mut offsets,
                padding,
            );
            if !preserve_affinity {
                placement::refine(
                    &children[node],
                    &relationships,
                    &sizes,
                    &mut offsets,
                    height,
                    padding,
                );
            }
            affinity::align(
                &children[node],
                affinities,
                &sizes,
                &mut offsets,
                height,
                padding,
            );
            for &child in &children[node] {
                offsets[child].x += padding;
                offsets[child].y += header_heights[node] + padding;
            }
            sizes[node] = (
                width.max(NODE_WIDTH) + padding * 2,
                header_heights[node]
                    + padding
                    + height
                    + if children[node].is_empty() {
                        0
                    } else {
                        padding
                    },
            );
        }
    }
    let (root_height, scopes) = super::scopes::pack(graph, &tree, &sizes, &mut offsets);
    let mut layout = Layout {
        scopes,
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
    for panel in &layout.scopes {
        layout.width = layout.width.max(panel.bounds.right() + 40);
    }
    layout
}

pub(super) fn header_heights(graph: &Graph, tree: &super::ContainmentTree) -> Vec<usize> {
    let numbered = super::relationship_markers::ends(graph);
    routing::incidents(graph, tree)
        .iter()
        .enumerate()
        .map(|(node, edges)| {
            let inferred =
                tree.parents[node].is_some_and(|parent| graph.derived_containment(parent, node));
            (NODE_HEIGHT + if inferred { 18 } else { 0 })
                .max(40 + super::relationship_markers::port_span(edges, &numbered))
        })
        .collect()
}
