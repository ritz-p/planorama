use crate::{layout::Layout, model::Graph};
use std::{collections::BTreeMap, fmt::Write};

pub(super) struct Masks {
    groups: Vec<Vec<usize>>,
    edges: Vec<Option<usize>>,
}

impl Masks {
    pub fn new(graph: &Graph, layout: &Layout<'_>) -> Self {
        let keys: Vec<_> = graph
            .edges
            .iter()
            .zip(&layout.paths)
            .map(|(edge, path)| {
                let mut nodes = if path.is_empty() {
                    Vec::new()
                } else {
                    layout.occluders(edge)
                };
                nodes.sort_by_key(|&node| &graph.nodes[node].entity.id);
                let key: Vec<_> = nodes
                    .iter()
                    .map(|&node| graph.nodes[node].entity.id.as_str())
                    .collect();
                (key, nodes)
            })
            .collect();
        let groups: BTreeMap<_, _> = keys
            .iter()
            .filter(|(key, _)| !key.is_empty())
            .cloned()
            .collect();
        let indexes: BTreeMap<_, _> = groups.keys().enumerate().map(|(i, key)| (key, i)).collect();
        let edges = keys
            .iter()
            .map(|(key, _)| indexes.get(key).copied())
            .collect();
        Self {
            groups: groups.into_values().collect(),
            edges,
        }
    }

    pub fn attribute(&self, edge: usize) -> String {
        self.edges[edge].map_or_else(String::new, |index| {
            format!(r#" mask="url(#container-occlusion-{index})""#)
        })
    }

    pub fn definitions(&self, layout: &Layout<'_>) -> String {
        let mut svg = String::new();
        for (index, nodes) in self.groups.iter().enumerate() {
            writeln!(svg, r#"<mask id="container-occlusion-{index}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="0" y="0" width="{}" height="{}" style="mask-type:luminance"><rect width="{}" height="{}" fill="white"/>"#, layout.width, layout.height, layout.width, layout.height).unwrap();
            for &node in nodes {
                let bounds = layout.bounds[node];
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="14" fill="black"/>"#,
                    bounds.origin.x.saturating_sub(2),
                    bounds.origin.y.saturating_sub(2),
                    bounds.width + 4,
                    bounds.height + 4
                )
                .unwrap();
            }
            svg.push_str("</mask>\n");
        }
        svg
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/occlusion.rs"]
mod tests;
