use crate::layout::{Bounds, Point, Port, Side, relationship_markers};
use crate::model::Directionality;
use crate::model::architecture::{Action, EdgeKind, Graph};
use std::collections::BTreeSet;
use std::fmt::Write;

const PADDING: usize = 16;
const LINE_HEIGHT: usize = 18;
const SAMPLE_NUMBER: usize = 12;
const SAMPLE_INSET: usize = 5;
const MIN_TEXT_WIDTH: usize = 208;
const MAX_ENTRY_WIDTH: usize = 640;
const MAX_CARD_WIDTH: usize = 960;
const MAX_PROSE_WIDTH: usize = 1040;
const COLUMN_GAP: usize = 24;
const ROW_GAP: usize = 16;

#[derive(Clone, Copy)]
struct Entry<'a> {
    kind: EdgeKind,
    name: &'a str,
    explanation: &'a str,
    directionality: Directionality,
}

impl Entry<'_> {
    fn key(&self) -> String {
        self.name.to_ascii_lowercase().replace(' ', "-")
    }

    fn minimum_width(&self) -> usize {
        3 * PADDING + sample_width() + MIN_TEXT_WIDTH.max(crate::view::text::width(self.name) * 8)
    }

    fn measure(&self, width: usize) -> MeasuredContent {
        let mut content = MeasuredContent {
            width,
            y: 0,
            items: Vec::new(),
        };
        content.entry(self);
        content
    }
}

const ENTRIES: [Entry<'static>; 5] = [
    Entry {
        kind: EdgeKind::Dependency,
        name: "Dependency",
        explanation: "Thin solid arrow from the referenced resource to the dependent resource; not exact Terraform execution order.",
        directionality: Directionality::Directed,
    },
    Entry {
        kind: EdgeKind::Association,
        name: "Association",
        explanation: "Dashed, undirected relationship. Matching badges identify the same Terraform-backed relationship at both attachment points.",
        directionality: Directionality::Undirected,
    },
    Entry {
        kind: EdgeKind::Association,
        name: "Directed Association",
        explanation: "Dashed line from a Route to its next hop. The destination arrow expresses direction; matching badges identify the same relationship.",
        directionality: Directionality::Directed,
    },
    Entry {
        kind: EdgeKind::Connection,
        name: "Connection",
        explanation: "Thicker dotted line shows an inferred, undirected connection; this does not guarantee actual network reachability.",
        directionality: Directionality::Undirected,
    },
    Entry {
        kind: EdgeKind::Containment,
        name: "Containment",
        explanation: "Nested boxes show containment; nesting replaces a redundant relationship line.",
        directionality: Directionality::Undirected,
    },
];

pub(super) struct MeasuredLegend {
    pub width: usize,
    pub height: usize,
    bounds: Bounds,
    content: Option<MeasuredContent>,
}

impl MeasuredLegend {
    pub fn new(graph: &Graph, available_width: usize) -> Self {
        Self::from_graphs([graph], available_width, 148)
    }

    pub fn empty(width: usize) -> Self {
        Self {
            width,
            height: 0,
            bounds: Bounds {
                origin: Point { x: 40, y: 0 },
                width: 0,
                height: 0,
            },
            content: None,
        }
    }

    pub fn from_graphs<'a>(
        graphs: impl IntoIterator<Item = &'a Graph>,
        available_width: usize,
        top: usize,
    ) -> Self {
        let edges: Vec<_> = graphs.into_iter().flat_map(|graph| &graph.edges).collect();
        let actions: BTreeSet<Option<Action>> = edges
            .iter()
            .filter(|edge| edge.kind != EdgeKind::Containment)
            .map(|edge| edge.change.as_ref().map(|change| change.action))
            .collect();
        if actions.is_empty() {
            return Self::empty(available_width);
        }
        let width = available_width.clamp(360, MAX_CARD_WIDTH + 80);
        let card_width = width - 80;
        let mut content = MeasuredContent {
            width: card_width,
            y: 44,
            items: Vec::new(),
        };
        content.text(
            "Line color shows the relationship's own Terraform action, independently of endpoint-card actions.",
            PADDING,
            false,
        );
        content.y += 8;
        content.actions(actions);
        content.text(
            "no independent change: no separate Terraform action is identified. unchanged: an explicit no-op action.",
            PADDING,
            false,
        );
        content.y += 16;
        let mut entries = ENTRIES.to_vec();
        if edges.iter().any(|e| e.kind == EdgeKind::Subscription) {
            entries.push(Entry {
                kind: EdgeKind::Subscription,
                name: "Subscription",
                explanation: "Directed topic-to-endpoint subscription proven by Terraform references. Delivery success and access policies are not verified.",
                directionality: Directionality::Directed,
            });
        }
        content.grid(&entries);
        content.text(
            "Hollow circles mark resource/card attachment points; neither end of an undirected line is a directional start. Filled internal dots mark bundle junctions.",
            PADDING,
            false,
        );
        content.y += 8;
        content.text(
            "Matching [N] badges identify the same Terraform-backed relationship and its Relationship Resources entry. They do not express direction or count connections. Reference-only Connections and Dependencies have no number.",
            PADDING,
            false,
        );
        Self {
            width,
            height: content.y + PADDING + 12,
            bounds: Bounds {
                origin: Point { x: 40, y: top },
                width: card_width,
                height: content.y + PADDING,
            },
            content: Some(content),
        }
    }

    pub fn render(&self) -> String {
        let Some(content) = &self.content else {
            return String::new();
        };
        let card_width = self.bounds.width;
        let card_height = self.bounds.height;
        let left = self.bounds.origin.x;
        let top = self.bounds.origin.y;
        format!(
            r##"<g id="relationship-legend" font-size="12" fill="#475569" transform="translate({left} {top})"><rect width="{card_width}" height="{card_height}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>
<text x="16" y="29" font-size="15" font-weight="700" fill="#334155">Relationship lines</text>
{}</g>
"##,
            content.render(),
        )
    }
}

struct MeasuredContent {
    width: usize,
    y: usize,
    items: Vec<MeasuredItem>,
}

impl MeasuredContent {
    fn grid(&mut self, entries: &[Entry<'_>]) {
        let Some(minimum) = entries.iter().map(Entry::minimum_width).max() else {
            return;
        };
        let columns = ((self.width + COLUMN_GAP) / (minimum + COLUMN_GAP))
            .clamp(1, 2)
            .min(entries.len());
        let cell_width = ((self.width - (columns - 1) * COLUMN_GAP) / columns).min(MAX_ENTRY_WIDTH);
        for row in entries.chunks(columns) {
            let cells: Vec<_> = row.iter().map(|entry| entry.measure(cell_width)).collect();
            let row_height = cells.iter().map(|cell| cell.y).max().unwrap_or(0);
            for (column, (entry, cell)) in row.iter().zip(cells).enumerate() {
                self.items.push(MeasuredItem::Group {
                    key: entry.key(),
                    bounds: Bounds {
                        origin: Point {
                            x: column * (cell_width + COLUMN_GAP),
                            y: self.y,
                        },
                        width: cell_width,
                        height: cell.y,
                    },
                    content: cell,
                });
            }
            self.y += row_height + ROW_GAP;
        }
    }

    fn entry(&mut self, entry: &Entry<'_>) {
        let sample_bottom = self.y + self.sample(entry.kind, &entry.key(), entry.directionality);
        let text_x = PADDING + sample_width() + PADDING;
        let text_x = if self.width.saturating_sub(text_x + PADDING) < MIN_TEXT_WIDTH {
            self.y = sample_bottom + 8;
            PADDING
        } else {
            text_x
        };
        self.text(entry.name, text_x, true);
        self.text(entry.explanation, text_x, false);
        self.y = self.y.max(sample_bottom);
    }

    fn sample(&mut self, kind: EdgeKind, key: &str, directionality: Directionality) -> usize {
        let start = Point {
            x: PADDING + SAMPLE_INSET,
            y: self.y + 2 + relationship_markers::HEIGHT / 2,
        };
        let end = Point {
            x: PADDING + sample_width() - SAMPLE_INSET,
            y: start.y,
        };
        let x = PADDING + (sample_width() - 44) / 2;
        self.items.push(MeasuredItem::Sample(MeasuredSample {
            kind,
            key: key.into(),
            directionality,
            start,
            end,
            outer: Bounds {
                origin: Point { x, y: self.y + 2 },
                width: 44,
                height: 28,
            },
            inner: Bounds {
                origin: Point {
                    x: x + 9,
                    y: self.y + 12,
                },
                width: 26,
                height: 12,
            },
            arrow_path: super::endpoints::arrow_path(start, end),
            badges: [
                relationship_markers::bounds(
                    SAMPLE_NUMBER,
                    Port {
                        point: start,
                        side: Side::Right,
                    },
                ),
                relationship_markers::bounds(
                    SAMPLE_NUMBER,
                    Port {
                        point: end,
                        side: Side::Left,
                    },
                ),
            ],
        }));
        if kind == EdgeKind::Containment {
            32
        } else {
            relationship_markers::HEIGHT + 4
        }
    }

    fn text(&mut self, text: &str, x: usize, bold: bool) {
        let limit = (self.width.min(MAX_PROSE_WIDTH) - x - PADDING) / 8;
        for line in wrap_words(text, limit) {
            self.items.push(MeasuredItem::Text {
                position: Point { x, y: self.y + 12 },
                bold,
                line,
            });
            self.y += LINE_HEIGHT;
        }
    }

    fn actions(&mut self, actions: BTreeSet<Option<Action>>) {
        let actions: BTreeSet<Option<Action>> = [None, Some(Action::Unchanged)]
            .into_iter()
            .chain(actions)
            .collect();
        let mut x = PADDING;
        for action in actions {
            let label = action.map_or("no independent change", super::label);
            let width = 34 + crate::view::text::width(label) * 8;
            if x > PADDING && x + width > self.width - PADDING {
                x = PADDING;
                self.y += 24;
            }
            self.items.push(MeasuredItem::Action {
                action,
                start: Point { x, y: self.y + 7 },
                end_x: x + 26,
                text: Point {
                    x: x + 34,
                    y: self.y + 12,
                },
            });
            x += width + 16;
        }
        self.y += 28;
    }
}

enum MeasuredItem {
    Text {
        position: Point,
        bold: bool,
        line: String,
    },
    Group {
        key: String,
        bounds: Bounds,
        content: MeasuredContent,
    },
    Action {
        action: Option<Action>,
        start: Point,
        end_x: usize,
        text: Point,
    },
    Sample(MeasuredSample),
}

struct MeasuredSample {
    kind: EdgeKind,
    key: String,
    directionality: Directionality,
    start: Point,
    end: Point,
    outer: Bounds,
    inner: Bounds,
    arrow_path: String,
    badges: [Bounds; 2],
}

impl MeasuredContent {
    fn render(&self) -> String {
        let mut svg = String::new();
        for item in &self.items {
            match item {
                MeasuredItem::Text {
                    position,
                    bold,
                    line,
                } => {
                    let weight = if *bold { " font-weight=\"700\"" } else { "" };
                    writeln!(
                        svg,
                        "<text x=\"{}\" y=\"{}\"{weight}>{}</text>",
                        position.x,
                        position.y,
                        super::escape(line)
                    )
                    .unwrap();
                }
                MeasuredItem::Group {
                    key,
                    bounds,
                    content,
                } => {
                    writeln!(
                        svg,
                        r#"<g data-legend-entry="{key}" transform="translate({} {})">"#,
                        bounds.origin.x, bounds.origin.y
                    )
                    .unwrap();
                    svg.push_str(&content.render());
                    svg.push_str("</g>\n");
                }
                MeasuredItem::Action {
                    action,
                    start,
                    end_x,
                    text,
                } => {
                    let name = action.map_or("none", super::label);
                    let label = action.map_or("no independent change", super::label);
                    let color = super::relationships::action_stroke(*action);
                    let style = super::relationships::style(EdgeKind::Dependency);
                    writeln!(svg, r#"<path data-legend-action="{name}" d="M {} {} H {end_x}" fill="none" stroke="{color}" {style}/><text x="{}" y="{}">{label}</text>"#, start.x, start.y, text.x, text.y).unwrap();
                }
                MeasuredItem::Sample(sample) => sample.render(&mut svg),
            }
        }
        svg
    }
}

impl MeasuredSample {
    fn render(&self, svg: &mut String) {
        let Self {
            kind,
            key,
            directionality,
            start,
            end,
            outer,
            inner,
            arrow_path,
            badges,
        } = self;
        let stroke = super::relationships::action_stroke(None);
        let style = super::relationships::style(*kind);
        writeln!(svg, r#"<g data-legend-sample="{key}">"#).unwrap();
        if *kind == EdgeKind::Containment {
            writeln!(svg, r#"<g data-legend-kind="containment" fill="none" stroke="{stroke}"><rect x="{}" y="{}" width="{}" height="{}" rx="4"/><rect x="{}" y="{}" width="{}" height="{}" rx="2"/></g>"#, outer.origin.x, outer.origin.y, outer.width, outer.height, inner.origin.x, inner.origin.y, inner.width, inner.height).unwrap();
            svg.push_str("</g>\n");
            return;
        }
        let arrow = super::relationships::arrow(*directionality, None);
        writeln!(svg, r#"<path data-legend-kind="{key}" data-directionality="{}" d="M {} {} H {}" fill="none" stroke="{stroke}" {style}{arrow}/>"#, directionality.as_str(), start.x, start.y, end.x).unwrap();
        svg.push_str(&super::endpoints::attachment_circle(
            *start,
            r#"data-legend-endpoint="source""#,
            stroke,
        ));
        if matches!(
            kind,
            EdgeKind::Association | EdgeKind::Connection | EdgeKind::Subscription
        ) {
            svg.push_str(&super::endpoints::attachment_circle(
                *end,
                r#"data-legend-endpoint="target""#,
                stroke,
            ));
            if *directionality == Directionality::Directed {
                writeln!(
                    svg,
                    r#"<path data-legend-arrow="true" d="{arrow_path}" fill="{stroke}"/>"#
                )
                .unwrap();
            }
        }
        if matches!(kind, EdgeKind::Association | EdgeKind::Subscription) {
            for (bounds, terminal) in badges.iter().zip(["source", "target"]) {
                writeln!(svg, r#"<g data-legend-badge="{terminal}">"#).unwrap();
                svg.push_str(&super::relationship_index::badge(
                    SAMPLE_NUMBER,
                    *bounds,
                    stroke,
                    style,
                ));
                svg.push_str("</g>\n");
            }
        }
        svg.push_str("</g>\n");
    }
}

fn sample_width() -> usize {
    2 * (SAMPLE_INSET + relationship_markers::GAP + relationship_markers::width(SAMPLE_NUMBER)) + 48
}

fn wrap_words(text: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty()
            && crate::view::text::width(&line) + 1 + crate::view::text::width(word) > limit
        {
            lines.push(std::mem::take(&mut line));
        }
        let parts = crate::view::text::wrap(word, limit);
        for (index, part) in parts.iter().enumerate() {
            if index > 0 {
                lines.push(std::mem::take(&mut line));
            } else if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(part);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
#[path = "../../tests/unit/svg/legend.rs"]
mod tests;
