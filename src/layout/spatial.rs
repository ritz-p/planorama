use super::{Band, Bounds, ContainmentTree, Layout, Point};
use crate::model::architecture::Graph;

pub(super) fn layout(graph: &Graph) -> Layout<'_> {
    let projected = crate::view::spatial::project(graph);
    let spatial = Layout::new(&projected.graph);
    let empty = Bounds {
        origin: Point { x: 0, y: 0 },
        width: 0,
        height: 0,
    };
    let mut bounds = vec![empty; graph.nodes.len()];
    let mut header_heights = vec![0; graph.nodes.len()];
    let mut parents = vec![None; graph.nodes.len()];
    let mut children = vec![Vec::new(); graph.nodes.len()];
    let mut keys = vec![0; graph.nodes.len()];
    for (new, &old) in projected.nodes.iter().enumerate() {
        bounds[old] = spatial.bounds[new];
        header_heights[old] = spatial.header_heights[new];
        parents[old] = spatial.containment.parents[new].map(|p| projected.nodes[p]);
        children[old] = spatial.containment.children[new]
            .iter()
            .map(|&c| projected.nodes[c])
            .collect();
        keys[old] = spatial.containment.keys[new];
    }
    let mut paths = vec![Vec::new(); graph.edges.len()];
    for (new, &old) in projected.edges.iter().enumerate() {
        paths[old] = spatial.paths[new].clone();
    }
    let mut layout = Layout {
        scopes: spatial.scopes,
        positions: bounds.iter().map(|b| b.origin).collect(),
        bounds,
        header_heights,
        containers: spatial
            .containers
            .iter()
            .map(|&i| projected.nodes[i])
            .collect(),
        containment: ContainmentTree {
            parents,
            children,
            keys,
            roots: spatial
                .containment
                .roots
                .iter()
                .map(|&i| projected.nodes[i])
                .collect(),
        },
        width: spatial.width,
        height: spatial.height,
        bands: spatial
            .bands
            .iter()
            .map(|band| Band {
                label: graph
                    .nodes
                    .iter()
                    .find(|n| n.module == band.label)
                    .expect("band belongs to a resource module")
                    .module
                    .as_str(),
                top: band.top,
                height: band.height,
            })
            .collect(),
        paths,
        junctions: spatial.junctions,
    };
    let mut embedded = false;
    for &parent in &projected.nodes {
        let mut y = layout.bounds[parent].origin.y
            + 106
            + crate::view::references::height(&graph.nodes[parent]);
        for card in crate::view::routes::cards(graph, parent) {
            let height = card.height();
            if let Some(node) = card.node {
                embedded = true;
                let child = graph
                    .nodes
                    .iter()
                    .position(|n| n.entity.id == node.entity.id)
                    .unwrap();
                layout.bounds[child] = Bounds {
                    origin: Point {
                        x: layout.bounds[parent].origin.x + 12,
                        y,
                    },
                    width: crate::view::routes::WIDTH,
                    height,
                };
                layout.positions[child] = layout.bounds[child].origin;
                layout.header_heights[child] = height;
                layout.containment.parents[child] = Some(parent);
                layout.containment.children[parent].push(child);
                layout.containment.keys[child] = graph
                    .nodes
                    .iter()
                    .filter(|n| n.entity.id < node.entity.id)
                    .count();
                if !layout.containers.contains(&parent) {
                    layout.containers.push(parent);
                }
                layout.header_heights[parent] =
                    94 + crate::view::references::height(&graph.nodes[parent]);
            }
            y += height + 12;
        }
    }
    if embedded {
        let mut routing = graph.clone();
        let mut indices = Vec::new();
        routing.edges = graph
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let visible = layout.bounds[e.from].width > 0 && layout.bounds[e.to].width > 0;
                let child = crate::view::routes::owner(graph, &graph.nodes[e.from]).is_some()
                    || crate::view::routes::owner(graph, &graph.nodes[e.to]).is_some();
                if visible && (!child || e.kind == crate::model::EdgeKind::Association) {
                    indices.push(i);
                    Some(e.clone())
                } else {
                    None
                }
            })
            .collect();
        super::containers::route(&routing, &mut layout, true);
        super::routing_shared::refinement::refine(&routing, &mut layout);
        let routed = std::mem::replace(&mut layout.paths, vec![Vec::new(); graph.edges.len()]);
        for (index, path) in indices.into_iter().zip(routed) {
            layout.paths[index] = path;
        }
    }
    layout
}
