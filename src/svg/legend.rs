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

pub(super) struct Legend {
    pub width: usize,
    pub height: usize,
    svg: String,
}

impl Legend {
    pub fn new(graph: &Graph, available_width: usize) -> Self {
        if !graph
            .edges
            .iter()
            .any(|edge| edge.kind != EdgeKind::Containment)
        {
            return Self {
                width: available_width,
                height: 0,
                svg: String::new(),
            };
        }
        let width = available_width.max(360);
        let card_width = (width - 80).min(1040);
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
        content.actions(graph);
        content.text(
            "no independent change: no separate Terraform action is identified. unchanged: an explicit no-op action.",
            PADDING,
            false,
        );
        content.y += 16;
        for (kind, name, explanation, directionality) in [
            (
                EdgeKind::Dependency,
                "Dependency",
                "Thin solid arrow from the referenced resource to the dependent resource; not exact Terraform execution order.",
                Directionality::Directed,
            ),
            (
                EdgeKind::Association,
                "Association",
                "Dashed, undirected relationship. Matching badges identify the same Terraform-backed relationship at both attachment points.",
                Directionality::Undirected,
            ),
            (
                EdgeKind::Association,
                "Directed Association",
                "Dashed line from a Route to its next hop. The destination arrow expresses direction; matching badges identify the same relationship.",
                Directionality::Directed,
            ),
            (
                EdgeKind::Connection,
                "Connection",
                "Thicker dotted line shows an inferred, undirected connection; this does not guarantee actual network reachability.",
                Directionality::Undirected,
            ),
            (
                EdgeKind::Containment,
                "Containment",
                "Nested boxes show containment; nesting replaces a redundant relationship line.",
                Directionality::Undirected,
            ),
        ] {
            content.entry(kind, name, explanation, directionality);
        }
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
            r##"<g id="relationship-legend" font-size="12" fill="#475569" transform="translate(40 148)"><rect width="{card_width}" height="{card_height}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>
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
    fn entry(
        &mut self,
        kind: EdgeKind,
        name: &str,
        explanation: &str,
        directionality: Directionality,
    ) {
        let key = name.to_ascii_lowercase().replace(' ', "-");
        writeln!(self.svg, r#"<g data-legend-entry="{key}">"#).unwrap();
        let sample_bottom = self.y + self.sample(kind, &key, directionality);
        let text_x = PADDING + sample_width() + PADDING;
        let text_x = if self.width.saturating_sub(text_x + PADDING) < MIN_TEXT_WIDTH {
            self.y = sample_bottom + 8;
            PADDING
        } else {
            text_x
        };
        self.text(name, text_x, true);
        self.text(explanation, text_x, false);
        self.y = self.y.max(sample_bottom) + 12;
        self.svg.push_str("</g>\n");
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
        let limit = (self.width - x - PADDING) / 8;
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

    fn actions(&mut self, graph: &Graph) {
        let actions: BTreeSet<Option<Action>> = [None, Some(Action::Unchanged)]
            .into_iter()
            .chain(
                graph
                    .edges
                    .iter()
                    .filter(|edge| edge.kind != EdgeKind::Containment)
                    .map(|edge| edge.change.as_ref().map(|change| change.action)),
            )
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
