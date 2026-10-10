use crate::layout::{Bounds, Layout};
use crate::model::{ArchitectureId, EdgeKind, Graph, RelationshipProvenance, TerraformEntityId};
use std::collections::BTreeMap;
use std::fmt::Write;

mod markers;

#[cfg(test)]
#[path = "../../tests/unit/svg/relationship_index.rs"]
mod tests;

type Key<'a> = (
    Vec<&'a TerraformEntityId>,
    &'a ArchitectureId,
    &'a ArchitectureId,
    EdgeKind,
);

pub(super) struct Entry<'a> {
    sources: Vec<&'a TerraformEntityId>,
    edges: Vec<usize>,
    stroke: &'static str,
    style: &'static str,
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
        let mut relationships = BTreeMap::new();
        for relationship in &graph.relationships {
            relationships
                .entry((&relationship.from, &relationship.to, relationship.kind))
                .or_insert_with(Vec::new)
                .push(relationship);
        }
        let mut grouped: BTreeMap<Key<'_>, Vec<usize>> = BTreeMap::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            if layout.paths.get(index).is_none_or(|path| path.len() < 2) {
                continue;
            }
            let from = &graph.nodes[edge.from].entity.id;
            let to = &graph.nodes[edge.to].entity.id;
            let Some(matching) = relationships.get(&(from, to, edge.kind)) else {
                continue;
            };
            for relationship in matching {
                let matches_change = edge.change.as_ref().map_or(matching.len() == 1, |change| relationship.provenance.iter().any(|p| matches!(p, RelationshipProvenance::Resource { change: source, .. } if source == change)));
                if !matches_change {
                    continue;
                }
                let mut sources: Vec<_> = relationship
                    .provenance
                    .iter()
                    .filter_map(|p| match p {
                        RelationshipProvenance::Resource { source, .. } => Some(source),
                        RelationshipProvenance::Reference { .. } => None,
                    })
                    .collect();
                sources.sort();
                sources.dedup();
                if !sources.is_empty() {
                    grouped
                        .entry((sources, from, to, edge.kind))
                        .or_default()
                        .push(index);
                }
            }
        }
        let entries: Vec<_> = grouped
            .into_iter()
            .map(|((sources, ..), mut edges)| {
                edges.sort_unstable();
                edges.dedup();
                let edge = &graph.edges[edges[0]];
                Entry {
                    sources,
                    edges,
                    stroke: super::relationships::stroke(edge),
                    style: super::relationships::style(edge.kind),
                }
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

    pub fn height(&self) -> usize {
        if self.entries.is_empty() {
            return 0;
        }
        60 + self
            .entries
            .iter()
            .map(|entry| self.rows(entry) * 20 + 8)
            .sum::<usize>()
    }

    fn line_limit(&self) -> usize {
        (self.width.saturating_sub(150) / 8).max(1)
    }

    fn rows(&self, entry: &Entry<'_>) -> usize {
        usize::from(entry.sources.len() > 1)
            + entry
                .sources
                .iter()
                .map(|source| wrap(&display(source), self.line_limit()).len())
                .sum::<usize>()
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

    pub fn marker_bounds(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.markers.iter().map(|marker| marker.bounds)
    }

    pub fn render(&self, top: usize) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut svg = format!(
            r##"<g data-relationship-index="true"><text x="40" y="{}" font-size="16" font-weight="700" fill="#334155">Relationship resources</text>
"##,
            top + 24
        );
        let mut y = top + 48;
        for (i, entry) in self.entries.iter().enumerate() {
            let number = i + 1;
            writeln!(svg, r##"<g data-relationship-entry="{number}">"##).unwrap();
            let bounds = Bounds {
                origin: crate::layout::Point { x: 52, y: y - 13 },
                width: crate::layout::relationship_markers::width(number),
                height: crate::layout::relationship_markers::HEIGHT,
            };
            svg.push_str(&badge(number, bounds, entry.stroke, entry.style));
            if entry.sources.len() > 1 {
                writeln!(svg, r##"<text x="100" y="{y}" font-size="12" fill="#334155">{} Terraform resources</text>"##, entry.sources.len()).unwrap();
                y += 20;
            }
            for source in &entry.sources {
                let text = display(source);
                writeln!(svg, r##"<text data-source-address="{}" font-size="12" fill="#334155"><title>{}</title>"##, super::escape(&source.address), super::escape(&text)).unwrap();
                for line in wrap(&text, self.line_limit()) {
                    writeln!(
                        svg,
                        "<tspan x=\"100\" y=\"{y}\" xml:space=\"preserve\">{}</tspan>",
                        super::escape(line)
                    )
                    .unwrap();
                    y += 20;
                }
                svg.push_str("</text>\n");
            }
            y += 8;
            svg.push_str("</g>\n");
        }
        svg.push_str("</g>\n");
        svg
    }
}

fn wrap(text: &str, limit: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut width = 0;
    for (offset, character) in text.char_indices() {
        let advance = if character.is_ascii() { 1 } else { 2 };
        if width > 0 && width + advance > limit {
            lines.push(&text[start..offset]);
            start = offset;
            width = 0;
        }
        width += advance;
    }
    lines.push(&text[start..]);
    lines
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
