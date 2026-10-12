use crate::model::Directionality;
use crate::model::architecture::{Action, EdgeKind, Graph};
use std::collections::BTreeSet;
use std::fmt::Write;

const PADDING: usize = 16;
const SAMPLE_TEXT_X: usize = 80;
const LINE_HEIGHT: usize = 18;

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
                "Dashed and usually undirected. A route next-hop association is directed from the route to its target.",
                Directionality::Undirected,
            ),
            (
                EdgeKind::Connection,
                "Connection",
                "Thicker dots show an inferred, undirected connection; this does not guarantee actual network reachability.",
                Directionality::Undirected,
            ),
        ] {
            let stroke = super::relationships::action_stroke(None);
            let style = super::relationships::style(kind);
            let arrow = super::relationships::arrow(directionality, None);
            writeln!(
                content.svg,
                r#"<path data-legend-kind="{}" data-directionality="{}" d="M 16 {} H 60" fill="none" stroke="{stroke}" {style}{arrow}/>"#,
                name.to_ascii_lowercase(),
                directionality.as_str(),
                content.y + 7,
            )
            .unwrap();
            content.text(name, SAMPLE_TEXT_X, true);
            content.text(explanation, SAMPLE_TEXT_X, false);
            content.y += 12;
        }
        let stroke = super::relationships::action_stroke(None);
        writeln!(
            content.svg,
            r#"<g data-legend-kind="containment" fill="none" stroke="{stroke}"><rect x="16" y="{}" width="44" height="28" rx="4"/><rect x="25" y="{}" width="26" height="12" rx="2"/></g>"#,
            content.y + 2,
            content.y + 12,
        )
        .unwrap();
        content.text("Containment", SAMPLE_TEXT_X, true);
        content.text(
            "Nested boxes show containment; nesting replaces a redundant relationship line.",
            SAMPLE_TEXT_X,
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
