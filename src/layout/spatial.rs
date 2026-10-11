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
    Layout {
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
    }
}
