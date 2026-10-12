use crate::layout::{Bounds, Layout};
use crate::model::{Graph, TerraformEntityId};
use crate::view::text::wrap_address;
use std::fmt::Write;

mod markers;

#[cfg(test)]
#[path = "../../tests/unit/svg/relationship_index.rs"]
mod tests;

pub(super) struct Entry<'a> {
    number: usize,
    sources: Vec<&'a TerraformEntityId>,
    edges: Vec<usize>,
    stroke: &'static str,
    style: &'static str,
    embedded: bool,
}

pub(super) struct Index<'a> {
    entries: Vec<Entry<'a>>,
    markers: Vec<Marker>,
    pub width: usize,
}

struct Marker {
    number: usize,
    bounds: Bounds,
    stroke: &'static str,
    style: &'static str,
}

impl<'a> Index<'a> {
    pub fn new(graph: &'a Graph, layout: &Layout<'_>) -> Self {
        let entries: Vec<_> = crate::layout::relationship_markers::entries(graph)
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let edges: Vec<_> = entry
                    .edges
                    .into_iter()
                    .filter(|&edge| layout.paths.get(edge).is_some_and(|path| path.len() >= 2))
                    .collect();
                if edges.is_empty() {
                    return None;
                }
                let edge = &graph.edges[edges[0]];
                Some(Entry {
                    number: index + 1,
                    embedded: crate::view::routes::owner(graph, &graph.nodes[edge.from]).is_some(),
                    sources: entry.sources,
                    edges,
                    stroke: super::relationships::stroke(edge),
                    style: super::relationships::style(edge.kind),
                })
            })
            .collect();
        let markers = markers::place(graph, layout, &entries);
        let width = markers.iter().fold(layout.width, |width, marker| {
            width.max(marker.bounds.right() + 20)
        });
        Self {
            entries,
            markers,
            width,
        }
    }

    pub fn markers(&self) -> String {
        let mut svg = String::new();
        for marker in &self.markers {
            let number = marker.number;
            let bounds = marker.bounds;
            let stroke = marker.stroke;
            let style = marker.style;
            writeln!(svg, "<g data-relationship-marker=\"{number}\"><title>Relationship resource [{number}]</title>").unwrap();
            svg.push_str(&badge(number, bounds, stroke, style));
            svg.push_str("</g>\n");
        }
        svg
    }

    pub fn number(&self, node: &crate::model::Node) -> Option<usize> {
        self.entries
            .iter()
            .find(|e| {
                e.embedded
                    && e.sources
                        .iter()
                        .any(|id| node.entity.provenance.contains(id))
            })
            .map(|entry| entry.number)
    }

    pub fn marker_bounds(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.markers.iter().map(|marker| marker.bounds)
    }

    pub fn cards(&self) -> Vec<super::annotations::Card> {
        const ENTRIES_PER_CARD: usize = 10;
        let badge_width = crate::layout::relationship_markers::width(
            self.entries.last().map_or(0, |entry| entry.number),
        );
        let text_x = 16 + badge_width + 12;
        let width = self
            .entries
            .iter()
            .flat_map(|entry| &entry.sources)
            .map(|source| {
                display(source)
                    .chars()
                    .map(|c| if c.is_ascii() { 8 } else { 16 })
                    .sum::<usize>()
                    + text_x
                    + 16
            })
            .max()
            .unwrap_or(360)
            .clamp(360, 640);
        self.entries.chunks(ENTRIES_PER_CARD).map(|entries| {
            let mut svg = String::from("<g data-relationship-index=\"true\">\n");
            let mut y = 28;
            for entry in entries {
                let number = entry.number;
                writeln!(svg, r#"<g data-relationship-entry="{number}">"#).unwrap();
                let bounds = Bounds {
                    origin: crate::layout::Point { x: 16, y: y - 13 },
                    width: crate::layout::relationship_markers::width(number),
                    height: crate::layout::relationship_markers::HEIGHT,
                };
                svg.push_str(&badge(number, bounds, entry.stroke, entry.style));
                if entry.sources.len() > 1 {
                    writeln!(svg, r##"<text x="{text_x}" y="{y}" font-size="12" fill="#334155">{} Terraform resources</text>"##, entry.sources.len()).unwrap();
                    y += 20;
                }
                for source in &entry.sources {
                    let text = display(source);
                    writeln!(svg, r##"<text data-source-address="{}" font-size="12" fill="#334155"><title>{}</title>"##, super::escape(&source.address), super::escape(&text)).unwrap();
                    for line in wrap_address(&text, (width - text_x - 16) / 8) {
                        writeln!(svg, "<tspan x=\"{text_x}\" y=\"{y}\" xml:space=\"preserve\">{}</tspan>", super::escape(line)).unwrap();
                        y += 20;
                    }
                    svg.push_str("</text>\n");
                }
                svg.push_str("</g>\n");
                y += 8;
            }
            svg.push_str("</g>\n");
            super::annotations::Card { width, height: y + 8, content: svg }
        }).collect()
    }
}

fn badge(number: usize, bounds: Bounds, stroke: &str, style: &str) -> String {
    format!(
        r##"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="#ffffff" stroke="{stroke}" {style}/><text x="{}" y="{}" text-anchor="middle" font-size="11" fill="#334155">[{number}]</text>
"##,
        bounds.origin.x,
        bounds.origin.y,
        bounds.width,
        bounds.height,
        bounds.origin.x + bounds.width / 2,
        bounds.origin.y + 13
    )
}

fn display(source: &TerraformEntityId) -> String {
    match &source.deposed_key {
        Some(key) => format!("{} (deposed: {key})", source.address),
        None => source.address.clone(),
    }
}
