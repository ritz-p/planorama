use crate::model::Graph;
use crate::view::routes::{WIDTH, cards};
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/routes.rs"]
mod tests;

pub(super) fn render(
    graph: &Graph,
    layout: &crate::layout::Layout<'_>,
    parent: usize,
    x: usize,
    mut y: usize,
    index: &super::relationship_index::Index<'_>,
    content: super::Content,
) -> String {
    let cards = cards(graph, parent);
    if cards.is_empty() {
        return String::new();
    }
    let radius = crate::view::geometry::ROUTE_RADIUS;
    let stroke = crate::view::geometry::ROUTE_STROKE as f64 / 4.0;
    let mut svg = format!(
        r##"<g data-routes="true"><text x="{x}" y="{y}" font-size="12" font-weight="700" fill="#475569">Routes</text>"##
    );
    y += 12;
    for mut card in cards {
        let height = card.height();
        let bounds = card.node.map_or(
            crate::layout::Bounds {
                origin: crate::layout::Point { x, y },
                width: WIDTH,
                height,
            },
            |node| {
                let child = graph
                    .nodes
                    .iter()
                    .position(|n| n.entity.id == node.entity.id)
                    .unwrap();
                layout.bounds[child]
            },
        );
        y += height + 12;
        let (x, y, width, height) = (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
        );
        if matches!(content, super::Content::ValueFree) {
            for row in &mut card.rows {
                if row.destination {
                    row.text = "Destination: omitted".into();
                }
            }
        }
        let node = card.node.unwrap_or(&graph.nodes[parent]);
        let (background, border) = super::color(node.action);
        if let Some(node) = card.node {
            write!(svg, r#"<g id="{}" data-architecture-id="{}" data-terraform-address="{}" data-qualified-address="{}" data-embedded-route="true"{}"#,
                super::identity::resource(node), super::escape(node.entity.id.as_str()), super::escape(&node.address),
                super::escape(&node.resource_address().qualified()), super::operations::attributes(&node.metadata)).unwrap();
            for (attribute, value) in [
                ("previous-address", &node.previous_address),
                ("deposed-key", &node.deposed_key),
            ] {
                if let Some(value) = value {
                    write!(svg, " data-{attribute}=\"{}\"", super::escape(value)).unwrap();
                }
            }
            svg.push('>');
        } else {
            write!(
                svg,
                r#"<g data-route-entry="{}">"#,
                if card.inline {
                    "inline"
                } else {
                    "resource-details"
                }
            )
            .unwrap();
        }
        let details = card
            .rows
            .iter()
            .filter(|row| !row.moved)
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        writeln!(svg, r#"<title>{}; {}{}{}{} </title><rect x="{x}" y="{y}" width="{width}" height="{height}" rx="{radius}" fill="{background}" stroke="{border}" stroke-width="{stroke}"/>"#,
            super::escape(&details), super::operations::label(node.action, &node.metadata), super::escape(&super::operations::details(&node.metadata)),
            node.previous_address.as_ref().map(|a| format!("; moved from: {}", super::escape(a))).unwrap_or_default(),
            node.deposed_key.as_ref().map(|a| format!("; deposed: {}", super::escape(a))).unwrap_or_default()).unwrap();
        if let Some(node) = card.node {
            svg.push_str(&super::entities::import_badge(node, x + 12, y + 10));
        }
        let mut line_y = y + 24 + card.badge_height();
        for row in card.rows {
            let next_hop = if row.text == "Next hop" {
                index.number(node).map(|n| format!("Next hop [{n}]"))
            } else {
                None
            };
            let text = next_hop.as_deref().unwrap_or(&row.text);
            let available = row.target.as_ref().and_then(|id| {
                graph
                    .nodes
                    .iter()
                    .find(|n| n.entity.provenance.contains(id))
            });
            if let Some(target) = available {
                write!(svg, "<a href=\"#{}\">", super::identity::resource(target)).unwrap();
            }
            for line in row.lines(text) {
                let (fill, annotation) = if row.moved {
                    ("#64748b", " data-move-annotation=\"true\"")
                } else {
                    ("#334155", "")
                };
                writeln!(svg, r##"<text x="{}" y="{line_y}" font-size="11" fill="{fill}" xml:space="preserve"{annotation}>{}</text>"##, x + 12, super::escape(line)).unwrap();
                line_y += 18;
            }
            if available.is_some() {
                svg.push_str("</a>");
            }
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</g>\n");
    svg
}
