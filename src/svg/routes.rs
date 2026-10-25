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
    origin: crate::layout::Point,
    index: &super::relationship_index::Index<'_>,
    content: super::Content,
    address_format: super::AddressFormat,
) -> String {
    let crate::layout::Point { x, mut y } = origin;
    let cards = cards(graph, parent);
    if cards.is_empty() {
        return String::new();
    }
    let radius = crate::view::geometry::ROUTE_RADIUS;
    let stroke = crate::view::geometry::ROUTE_STROKE as f64 / 4.0;
    let members = cards
        .iter()
        .any(|card| card.node.is_some_and(|node| node.member_owner.is_some()));
    let section = if members { "cluster-members" } else { "routes" };
    let heading = if members {
        cards
            .iter()
            .find_map(|card| card.node?.member_owner.as_ref().map(|owner| owner.section))
            .unwrap_or("Members")
    } else {
        "Routes"
    };
    let mut svg = format!(
        r##"<g data-{section}="true"><text x="{x}" y="{y}" font-size="12" font-weight="700" fill="#475569">{heading}</text>"##
    );
    y += 12;
    for mut card in cards {
        if let Some(node) = card.node.filter(|node| node.member_owner.is_some()) {
            card.rows[0].text = crate::view::routes::heading(
                node,
                matches!(address_format, super::AddressFormat::Qualified),
            );
        }
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
            let kind = if node.member_owner.is_some() {
                "member"
            } else {
                "route"
            };
            write!(svg, r#"<g id="{}" data-architecture-id="{}" data-terraform-address="{}" data-qualified-address="{}" data-embedded-{kind}="true"{}"#,
                super::identity::resource(node), super::escape(node.entity.id.as_str()), super::escape(&node.address),
                super::escape(&node.resource_address().qualified()), super::operations::attributes(&node.metadata)).unwrap();
            if let Some(owner) = &node.member_owner {
                write!(
                    svg,
                    " data-member-owner=\"{}\"",
                    super::escape(&owner.target.address)
                )
                .unwrap();
            }
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
        let heading_height = card.type_heading_height();
        if let Some(heading) = card.type_heading() {
            svg.push_str(r##"<text data-resource-type-heading="true" font-size="12" font-weight="700" fill="#334155">"##);
            for (line, text) in
                crate::view::text::wrap_address(&heading, crate::view::routes::LINE_LIMIT)
                    .iter()
                    .enumerate()
            {
                write!(
                    svg,
                    r#"<tspan x="{}" y="{}" xml:space="preserve">{}</tspan>"#,
                    x + 12,
                    y + 24 + line * 18,
                    super::escape(text)
                )
                .unwrap();
            }
            svg.push_str("</text>");
        }
        if let Some(node) = card.node {
            svg.push_str(&super::entities::import_badge(
                node,
                x + 12,
                y + 10 + heading_height,
            ));
            if node.member_owner.is_some() {
                let icon_y = y + 10 + heading_height + crate::view::card::import_height(node);
                if let Some(icon) = super::icons::for_node(node) {
                    svg.push_str(&super::icons::render(icon, x + 12, icon_y));
                }
                svg.push_str(&super::icons::service_name(
                    node,
                    x + width - 8,
                    icon_y,
                    width - 24,
                ));
            }
        }
        let mut line_y = y + 24 + heading_height + card.badge_height();
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
            let relationship_target = next_hop
                .as_ref()
                .and_then(|_| index.number(node))
                .map(|number| index.target(number));
            if let Some(target) = relationship_target {
                write!(
                    svg,
                    "<a href=\"#{target}\" data-relationship-select=\"true\">"
                )
                .unwrap();
            } else if let Some(target) = available {
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
            if relationship_target.is_some() || available.is_some() {
                svg.push_str("</a>");
            }
        }
        if node.member_owner.is_some() {
            svg.push_str(&super::references::render(
                graph,
                node,
                x + 12,
                line_y,
                width - 24,
            ));
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</g>\n");
    svg
}
