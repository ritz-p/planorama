use crate::layout::{Bounds, Layout};
use crate::model::{ArchitectureId, EdgeKind, Graph, RelationshipProvenance, TerraformEntityId};
use crate::view::text::wrap_address;
use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Default)]
struct Group<'a> {
    edges: Vec<usize>,
    previous_addresses: BTreeMap<&'a TerraformEntityId, BTreeSet<&'a str>>,
}

pub(super) struct Entry<'a> {
    sources: Vec<&'a TerraformEntityId>,
    previous_addresses: BTreeMap<&'a TerraformEntityId, BTreeSet<&'a str>>,
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
        let mut relationships = BTreeMap::new();
        for relationship in &graph.relationships {
            relationships
                .entry((&relationship.from, &relationship.to, relationship.kind))
                .or_insert_with(Vec::new)
                .push(relationship);
        }
        let mut grouped: BTreeMap<Key<'_>, Group<'_>> = BTreeMap::new();
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
                    let group = grouped.entry((sources, from, to, edge.kind)).or_default();
                    group.edges.push(index);
                    for provenance in &relationship.provenance {
                        if let RelationshipProvenance::Resource { source, change } = provenance {
                            if let Some(previous) = change.previous_address.as_deref() {
                                group
                                    .previous_addresses
                                    .entry(source)
                                    .or_default()
                                    .insert(previous);
                            }
                        }
                    }
                }
            }
        }
        let entries: Vec<_> = grouped
            .into_iter()
            .map(|((sources, ..), group)| {
                let Group {
                    mut edges,
                    previous_addresses,
                } = group;
                edges.sort_unstable();
                edges.dedup();
                let edge = &graph.edges[edges[0]];
                Entry {
                    embedded: crate::view::routes::owner(graph, &graph.nodes[edge.from]).is_some(),
                    sources,
                    previous_addresses,
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
            .position(|e| {
                e.embedded
                    && e.sources
                        .iter()
                        .any(|id| node.entity.provenance.contains(id))
            })
            .map(|i| i + 1)
    }

    pub fn marker_bounds(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.markers.iter().map(|marker| marker.bounds)
    }

    pub fn cards(&self) -> Vec<super::annotations::Card> {
        const ENTRIES_PER_CARD: usize = 10;
        let badge_width = crate::layout::relationship_markers::width(self.entries.len());
        let text_x = 16 + badge_width + 12;
        let width = self
            .entries
            .iter()
            .flat_map(|entry| {
                entry.sources.iter().flat_map(move |source| {
                    std::iter::once(display(source)).chain(
                        entry
                            .previous_addresses
                            .get(source)
                            .into_iter()
                            .flatten()
                            .map(|previous| crate::view::text::moved_from(previous)),
                    )
                })
            })
            .map(|text| {
                text.chars()
                    .map(|c| if c.is_ascii() { 8 } else { 16 })
                    .sum::<usize>()
                    + text_x
                    + 16
            })
            .max()
            .unwrap_or(360)
            .clamp(360, 640);
        let entries: Vec<_> = self.entries.iter().enumerate().collect();
        entries.chunks(ENTRIES_PER_CARD).map(|entries| {
            let mut svg = String::from("<g data-relationship-index=\"true\">\n");
            let mut y = 28;
            for &(offset, entry) in entries {
                let number = offset + 1;
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
                    if let Some(previous_addresses) = entry.previous_addresses.get(source) {
                        for previous in previous_addresses {
                            let (annotation, height) = super::operations::moved_from(
                                previous, text_x, y, (width - text_x - 16) / 8,
                            );
                            svg.push_str(&annotation);
                            y += height;
                        }
                    }
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
