use crate::layout::{Point, Port, Side, relationship_markers};
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

    fn measure(&self, width: usize) -> Content {
        let mut content = Content {
            width,
            y: 0,
            svg: String::new(),
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

pub(super) struct Legend {
    pub width: usize,
    pub height: usize,
    svg: String,
}

impl Legend {
    pub fn new(graph: &Graph, available_width: usize) -> Self {
        Self::from_graphs([graph], available_width, 148)
    }

    pub fn empty(width: usize) -> Self {
        Self {
            width,
            height: 0,
            svg: String::new(),
        }
    }

    pub fn from_graphs<'a>(
        graphs: impl IntoIterator<Item = &'a Graph>,
        available_width: usize,
        top: usize,
    ) -> Self {
        let actions: BTreeSet<Option<Action>> = graphs
            .into_iter()
            .flat_map(|graph| &graph.edges)
            .filter(|edge| edge.kind != EdgeKind::Containment)
            .map(|edge| edge.change.as_ref().map(|change| change.action))
            .collect();
        if actions.is_empty() {
            return Self::empty(available_width);
        }
        let width = available_width.max(360);
        let card_width = width - 80;
        let mut content = Content {
            width: card_width,
            y: 44,
            svg: String::from(
                "<text x=\"16\" y=\"29\" font-size=\"15\" font-weight=\"700\" fill=\"#334155\">Relationship lines</text>\n",
            ),
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
        content.grid(&ENTRIES);
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
        let card_height = content.y + PADDING;
        let svg = format!(
            r##"<g id="relationship-legend" font-size="12" fill="#475569" transform="translate(40 {top})"><rect width="{card_width}" height="{card_height}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>
{}</g>
"##,
            content.svg,
        );
        Self {
            width,
            height: card_height + 12,
            svg,
        }
    }

    pub fn render(&self) -> &str {
        &self.svg
    }
}

struct Content {
    width: usize,
    y: usize,
    svg: String,
}

impl Content {
    fn grid(&mut self, entries: &[Entry<'_>]) {
        let Some(minimum) = entries.iter().map(Entry::minimum_width).max() else {
            return;
        };
        let columns = ((self.width + COLUMN_GAP) / (minimum + COLUMN_GAP))
            .max(1)
            .min(entries.len());
        let cell_width = ((self.width - (columns - 1) * COLUMN_GAP) / columns).min(MAX_ENTRY_WIDTH);
        for row in entries.chunks(columns) {
            let cells: Vec<_> = row.iter().map(|entry| entry.measure(cell_width)).collect();
            let row_height = cells.iter().map(|cell| cell.y).max().unwrap_or(0);
            for (column, (entry, cell)) in row.iter().zip(cells).enumerate() {
                writeln!(
                    self.svg,
                    r#"<g data-legend-entry="{}" transform="translate({} {})">"#,
                    entry.key(),
                    column * (cell_width + COLUMN_GAP),
                    self.y,
                )
                .unwrap();
                self.svg.push_str(&cell.svg);
                self.svg.push_str("</g>\n");
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
        let stroke = super::relationships::action_stroke(None);
        let style = super::relationships::style(kind);
        writeln!(self.svg, r#"<g data-legend-sample="{key}">"#).unwrap();
        if kind == EdgeKind::Containment {
            let x = PADDING + (sample_width() - 44) / 2;
            writeln!(
                self.svg,
                r#"<g data-legend-kind="containment" fill="none" stroke="{stroke}"><rect x="{x}" y="{}" width="44" height="28" rx="4"/><rect x="{}" y="{}" width="26" height="12" rx="2"/></g>"#,
                self.y + 2,
                x + 9,
                self.y + 12,
            )
            .unwrap();
            self.svg.push_str("</g>\n");
            return 32;
        }
        let start = Point {
            x: PADDING + SAMPLE_INSET,
            y: self.y + 2 + relationship_markers::HEIGHT / 2,
        };
        let end = Point {
            x: PADDING + sample_width() - SAMPLE_INSET,
            y: start.y,
        };
        let arrow = super::relationships::arrow(directionality, None);
        writeln!(
            self.svg,
            r#"<path data-legend-kind="{key}" data-directionality="{}" d="M {} {} H {}" fill="none" stroke="{stroke}" {style}{arrow}/>"#,
            directionality.as_str(),
            start.x,
            start.y,
            end.x,
        )
        .unwrap();
        self.svg.push_str(&super::endpoints::attachment_circle(
            start,
            r#"data-legend-endpoint="source""#,
            stroke,
        ));
        if matches!(kind, EdgeKind::Association | EdgeKind::Connection) {
            self.svg.push_str(&super::endpoints::attachment_circle(
                end,
                r#"data-legend-endpoint="target""#,
                stroke,
            ));
            if directionality == Directionality::Directed {
                let path = super::endpoints::arrow_path(start, end);
                writeln!(
                    self.svg,
                    r#"<path data-legend-arrow="true" d="{path}" fill="{stroke}"/>"#,
                )
                .unwrap();
            }
        }
        if kind == EdgeKind::Association {
            for (point, side, terminal) in
                [(start, Side::Right, "source"), (end, Side::Left, "target")]
            {
                writeln!(self.svg, r#"<g data-legend-badge="{terminal}">"#).unwrap();
                self.svg.push_str(&super::relationship_index::badge(
                    SAMPLE_NUMBER,
                    relationship_markers::bounds(SAMPLE_NUMBER, Port { point, side }),
                    stroke,
                    style,
                ));
                self.svg.push_str("</g>\n");
            }
        }
        self.svg.push_str("</g>\n");
        relationship_markers::HEIGHT + 4
    }

    fn text(&mut self, text: &str, x: usize, bold: bool) {
        let weight = if bold { " font-weight=\"700\"" } else { "" };
        let limit = (self.width.min(MAX_PROSE_WIDTH) - x - PADDING) / 8;
        for line in wrap_words(text, limit) {
            writeln!(
                self.svg,
                "<text x=\"{x}\" y=\"{}\"{weight}>{}</text>",
                self.y + 12,
                super::escape(&line),
            )
            .unwrap();
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
            let name = action.map_or("none", super::label);
            let label = action.map_or("no independent change", super::label);
            let color = super::relationships::action_stroke(action);
            let style = super::relationships::style(EdgeKind::Dependency);
            let width = 34 + crate::view::text::width(label) * 8;
            if x > PADDING && x + width > self.width - PADDING {
                x = PADDING;
                self.y += 24;
            }
            writeln!(
                self.svg,
                r#"<path data-legend-action="{name}" d="M {x} {} H {}" fill="none" stroke="{color}" {style}/><text x="{}" y="{}">{label}</text>"#,
                self.y + 7,
                x + 26,
                x + 34,
                self.y + 12,
            )
            .unwrap();
            x += width + 16;
        }
        self.y += 28;
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
