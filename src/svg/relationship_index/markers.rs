use super::{Entry, Graph, Layout, Marker};
use crate::layout::{Port, Side, relationship_markers};

pub(super) fn place(graph: &Graph, layout: &Layout<'_>, entries: &[Entry<'_>]) -> Vec<Marker> {
    let mut markers = Vec::new();
    for entry in entries {
        let number = entry.number;
        for &edge in &entry.edges {
            let path = &layout.paths[edge];
            let target_segment = path
                .windows(2)
                .rev()
                .find(|s| s[0] != s[1])
                .map(|s| (s[0], s[1]));
            let source_segment = (graph.edges[edge].directionality()
                == crate::model::Directionality::Undirected)
                .then(|| path.windows(2).find(|s| s[0] != s[1]).map(|s| (s[1], s[0])))
                .flatten();
            for segment in [target_segment, source_segment] {
                let Some((before, target)) = segment else {
                    continue;
                };
                let side = if before.x < target.x {
                    Side::Left
                } else if before.x > target.x {
                    Side::Right
                } else if before.y < target.y {
                    Side::Top
                } else {
                    Side::Bottom
                };
                markers.push(Marker {
                    number,
                    stroke: super::super::relationships::stroke(&graph.edges[edge]),
                    style: super::super::relationships::style(graph.edges[edge].kind),
                    bounds: relationship_markers::bounds(
                        number,
                        Port {
                            point: target,
                            side,
                        },
                    ),
                });
            }
        }
    }
    markers
}
