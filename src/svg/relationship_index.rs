use crate::layout::{Bounds, Layout};
use crate::model::{Graph, TerraformEntityId};
use crate::view::text::wrap_address;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

mod markers;

#[cfg(test)]
#[path = "../../tests/unit/svg/relationship_index.rs"]
mod tests;

pub(super) struct Entry<'a> {
    id: String,
    context: Vec<Context>,
    actions: BTreeMap<&'a TerraformEntityId, BTreeSet<String>>,
    scopes: BTreeMap<&'a TerraformEntityId, BTreeSet<String>>,
    number: usize,
    sources: Vec<&'a TerraformEntityId>,
    previous_addresses: BTreeMap<&'a TerraformEntityId, BTreeSet<&'a str>>,
    edges: Vec<usize>,
    stroke: &'static str,
    style: &'static str,
    embedded: bool,
}

struct Context {
    heading: String,
    directed: bool,
    endpoints: [(String, Option<String>); 2],
    scopes: [String; 2],
}

impl Context {
    fn endpoint_label(&self, index: usize) -> String {
        let (text, target) = &self.endpoints[index];
        let direction = if self.directed {
            if index == 0 { "From: " } else { "To: " }
        } else {
            ""
        };
        let status = if target.is_none() {
            " (unavailable in this view)"
        } else {
            ""
        };
        let scope = if target.is_some() {
            format!(" · {}", self.scopes[index])
        } else {
            String::new()
        };
        format!("- {direction}{text}{status}{scope}")
    }
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
                    .iter()
                    .copied()
                    .filter(|&edge| layout.paths.get(edge).is_some_and(|path| path.len() >= 2))
                    .collect();
                if edges.is_empty() {
                    return None;
                }
                let edge = &graph.edges[edges[0]];
                let pairs: BTreeSet<_> = entry
                    .edges
                    .iter()
                    .map(|&i| {
                        let edge = &graph.edges[i];
                        (edge.from, edge.to, edge.kind, edge.directionality())
                    })
                    .collect();
                let mut identities: Vec<_> = pairs
                    .iter()
                    .map(|&(from, to, kind, direction)| {
                        (
                            graph.nodes[from].entity.id.as_str(),
                            graph.nodes[to].entity.id.as_str(),
                            format!("{kind:?}").to_lowercase(),
                            direction.as_str(),
                        )
                    })
                    .collect();
                identities.sort();
                let key = serde_json::json!([
                    entry
                        .sources
                        .iter()
                        .map(|source| (&source.address, &source.deposed_key))
                        .collect::<Vec<_>>(),
                    identities
                ])
                .to_string();
                let mut id = String::from("relationship-");
                for byte in key.bytes() {
                    write!(id, "{byte:02x}").unwrap();
                }
                let mut context: Vec<_> = pairs
                    .into_iter()
                    .map(|(from, to, kind, direction)| {
                        let directed = direction == crate::model::Directionality::Directed;
                        Context {
                            directed,
                            scopes: [from, to].map(|i| crate::view::scope::label(graph.nodes[i].entity.scope.as_ref(), &graph.nodes[i].provider)),
                            heading: format!(
                                "- {} ({}, {})",
                                if directed {
                                    "From → To"
                                } else if matches!(
                                    kind,
                                    crate::model::EdgeKind::Dependency
                                        | crate::model::EdgeKind::Containment
                                ) {
                                    "Endpoints"
                                } else {
                                    "Connects"
                                },
                                format!("{kind:?}").to_lowercase(),
                                direction.as_str()
                            ),
                            endpoints: [from, to].map(|i| {
                                let node = &graph.nodes[i];
                                (
                                    display_parts(&node.address, node.deposed_key.as_deref()),
                                    Some(super::identity::resource(node)),
                                )
                            }),
                        }
                    })
                    .collect();
                context.sort_by(|a, b| (&a.heading, &a.endpoints).cmp(&(&b.heading, &b.endpoints)));
                let mut actions: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
                let mut scopes: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
                for relationship in &graph.relationships {
                    if !entry.edges.iter().any(|&i| {
                        let edge = &graph.edges[i];
                        relationship.from == graph.nodes[edge.from].entity.id
                            && relationship.to == graph.nodes[edge.to].entity.id
                            && relationship.kind == edge.kind
                            && edge.change.as_ref().is_none_or(|change| {
                                relationship.provenance.iter().any(|provenance| {
                                    matches!(provenance, crate::model::RelationshipProvenance::Resource { change: source_change, .. } if source_change == change)
                                })
                            })
                    }) {
                        continue;
                    }
                    let mut sources: Vec<_> = relationship.provenance.iter().filter_map(|provenance| {
                        match provenance {
                            crate::model::RelationshipProvenance::Resource { source, .. } => Some(source),
                            crate::model::RelationshipProvenance::Reference { .. } => None,
                        }
                    }).collect();
                    sources.sort();
                    sources.dedup();
                    if sources != entry.sources { continue; }
                    for provenance in &relationship.provenance {
                        if let crate::model::RelationshipProvenance::Resource { source, change, deployment } =
                            provenance
                        {
                            scopes.entry(source).or_default().insert(deployment.as_ref().map_or_else(
                                || "Scope unavailable".into(),
                                |deployment| crate::view::scope::label(deployment.scope.as_ref(), &deployment.provider),
                            ));
                            actions.entry(source).or_default().insert(
                                super::operations::label(change.action, &change.metadata),
                            );
                        }
                    }
                }
                Some(Entry {
                    id,
                    context,
                    actions,
                    scopes,
                    number: index + 1,
                    embedded: crate::view::routes::owner(graph, &graph.nodes[edge.from]).is_some(),
                    sources: entry.sources,
                    previous_addresses: entry.previous_addresses,
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
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.number == number)
                .unwrap();
            writeln!(svg, "<g data-relationship-marker=\"{number}\"><a href=\"#{}\" data-relationship-select=\"true\" aria-label=\"{}\"><title>{}</title>", entry.id, super::escape(&entry.title()), super::escape(&entry.title())).unwrap();
            svg.push_str(&badge(number, bounds, stroke, style));
            svg.push_str("</a></g>\n");
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

    pub fn edge_attributes(&self, edge: usize) -> String {
        let ids = self
            .entries
            .iter()
            .filter(|entry| entry.edges.contains(&edge))
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if ids.is_empty() {
            String::new()
        } else {
            format!(" data-relationship-ids=\"{ids}\"")
        }
    }

    pub fn target(&self, number: usize) -> &str {
        &self
            .entries
            .iter()
            .find(|entry| entry.number == number)
            .unwrap()
            .id
    }

    pub fn cards(&self) -> Vec<super::annotations::Card> {
        const ENTRIES_PER_CARD: usize = 5;
        let badge_width = crate::layout::relationship_markers::width(
            self.entries.last().map_or(0, |entry| entry.number),
        );
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
            .chain(
                self.entries
                    .iter()
                    .flat_map(|entry| entry.scopes.values())
                    .flatten()
                    .map(|scope| {
                        crate::view::text::width(&format!("Resource scope: {scope}")) * 8
                            + text_x
                            + 16
                    }),
            )
            .chain(
                self.entries
                    .iter()
                    .flat_map(|entry| entry.context.iter())
                    .flat_map(|context| {
                        std::iter::once(
                            crate::view::text::width(&context.heading) * 8 + text_x + 32,
                        )
                        .chain((0..2).map(move |i| {
                            crate::view::text::width(&context.endpoint_label(i)) * 8 + text_x + 48
                        }))
                    }),
            )
            .max()
            .unwrap_or(360)
            .clamp(360, 640);
        self.entries.chunks(ENTRIES_PER_CARD).map(|entries| {
            let mut svg = String::from("<g data-relationship-index=\"true\">\n");
            let mut y = 28;
            for entry in entries {
                let number = entry.number;
                writeln!(svg, r##"<g data-relationship-entry="{number}" id="{}"><title>{}</title><a href="#{}" data-relationship-select="true" aria-label="{}">"##, entry.id, super::escape(&entry.title()), entry.id, super::escape(&entry.title())).unwrap();
                let bounds = Bounds {
                    origin: crate::layout::Point { x: 16, y: y - 13 },
                    width: crate::layout::relationship_markers::width(number),
                    height: crate::layout::relationship_markers::HEIGHT,
                };
                svg.push_str(&badge(number, bounds, entry.stroke, entry.style));
                svg.push_str("</a>\n");
                if entry.sources.len() > 1 {
                    writeln!(svg, r##"<text x="{text_x}" y="{y}" font-size="12" fill="#334155">{} Terraform resources</text>"##, entry.sources.len()).unwrap();
                    y += 20;
                }
                for source in &entry.sources {
                    let text = display(source);
                    writeln!(svg, r##"<text data-source-address="{}" data-source-identity="{}" font-size="12" font-weight="600" fill="#334155"><title>{}</title>"##, super::escape(&source.address), super::identity::source(source).replacen("resource-", "terraform:", 1), super::escape(&text)).unwrap();
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
                    if let Some(actions) = entry.actions.get(source) {
                        text_row(&mut svg, &format!("Action: {}", actions.iter().cloned().collect::<Vec<_>>().join(", ")), text_x, &mut y, width, None);
                    }
                    if let Some(scopes) = entry.scopes.get(source) {
                        for scope in scopes {
                            text_row(&mut svg, &format!("Resource scope: {scope}"), text_x, &mut y, width, None);
                        }
                    }
                }
                for context in &entry.context {
                    text_row(&mut svg, &context.heading, text_x + 16, &mut y, width, None);
                    for (i, (_, target)) in context.endpoints.iter().enumerate() {
                        text_row(&mut svg, &context.endpoint_label(i), text_x + 32, &mut y, width, target.as_deref());
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

impl Entry<'_> {
    fn title(&self) -> String {
        format!(
            "Relationship [{}]: {}",
            self.number,
            self.context
                .iter()
                .map(|context| format!(
                    "{}: {} / {}",
                    context.heading.trim_start_matches("- "),
                    context.endpoints[0].0,
                    context.endpoints[1].0
                ))
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}

fn text_row(
    svg: &mut String,
    text: &str,
    x: usize,
    y: &mut usize,
    width: usize,
    target: Option<&str>,
) {
    if let Some(target) = target {
        write!(
            svg,
            "<a href=\"#{target}\" data-relationship-endpoint=\"true\" aria-label=\"{}\">",
            super::escape(text)
        )
        .unwrap();
    }
    svg.push_str("<text font-size=\"12\" fill=\"#334155\">");
    for line in wrap_address(text, (width - x - 16) / 8) {
        write!(
            svg,
            "<tspan x=\"{x}\" y=\"{y}\" xml:space=\"preserve\">{}</tspan>",
            super::escape(line)
        )
        .unwrap();
        *y += 20;
    }
    svg.push_str("</text>\n");
    if target.is_some() {
        svg.push_str("</a>\n");
    }
}

pub(super) fn badge(number: usize, bounds: Bounds, stroke: &str, style: &str) -> String {
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
    display_parts(&source.address, source.deposed_key.as_deref())
}

fn display_parts(address: &str, deposed: Option<&str>) -> String {
    match deposed {
        Some(key) => format!("{address} (deposed: {key})"),
        None => address.to_owned(),
    }
}
