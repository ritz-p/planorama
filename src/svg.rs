//! SVG presentation of resolved architecture and layout; no semantic inference.
mod checks;
mod components;
mod containers;
mod entities;
mod icons;
mod identity;
mod legend;
mod operations;
mod paths;
mod provenance;
mod relationships;
mod roles;
mod states;
mod style;
pub use states::render_states;
#[cfg(test)]
#[path = "../tests/unit/svg.rs"]
mod tests;

use crate::layout::Layout;
use crate::model::architecture::{Action, EdgeKind, Graph};
use std::fmt::Write;
use style::{color, label, shorten};

#[derive(Clone, Copy, Default)]
pub enum AddressFormat {
    #[default]
    Qualified,
    Terraform,
}

fn escape(value: &str) -> String {
    value
        .chars()
        .filter(|&c| c >= ' ' || matches!(c, '\n' | '\r' | '\t'))
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            _ => c.to_string(),
        })
        .collect()
}

pub(crate) fn render(graph: &Graph, layout: &Layout<'_>) -> String {
    render_with_format(graph, layout, AddressFormat::Qualified)
}

fn dimensions(graph: &Graph, layout: &Layout<'_>) -> (usize, usize) {
    (
        layout.width,
        layout.height
            + if graph.checks.is_empty() { 0 } else { 24 }
            + components::height(graph)
            + legend::height(graph),
    )
}

pub(crate) fn render_with_format(
    graph: &Graph,
    layout: &Layout<'_>,
    address_format: AddressFormat,
) -> String {
    let check_offset = if graph.checks.is_empty() { 0 } else { 24 };
    let (width, height) = dimensions(graph, layout);
    let membership = components::membership(graph);
    let (description, summary) = relationships::captions(graph);
    let markers = relationships::markers(graph);
    let icon_definitions = icons::definitions(&graph.nodes);
    let semantic_edges = graph
        .edges
        .iter()
        .any(|edge| edge.kind != EdgeKind::Dependency);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">
<title id="title">Terraform plan</title>
<desc id="description">{description} Thin solid: dependency. Dashed: association. Thick dotted: connection. Nested boxes: containment. Edge color preserves the Terraform action when available.</desc>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8"/></marker>{markers}{icon_definitions}</defs>
<rect width="100%" height="100%" fill="#ffffff"/>
<g font-family="ui-monospace, SFMono-Regular, Consolas, monospace">
<text x="40" y="45" font-size="25" font-weight="700" fill="#0f172a">Terraform plan</text>
<text x="40" y="73" font-size="13" fill="#475569">{summary}</text>
"##,
    );
    svg.push_str(&provenance::metadata(graph));
    for (i, action) in [
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ]
    .iter()
    .enumerate()
    {
        let x = 40 + i * 140;
        let (background, border) = color(*action);
        let count = graph.nodes.iter().filter(|n| n.action == *action).count()
            + graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.change
                        .as_ref()
                        .is_some_and(|change| change.action == *action)
                })
                .count();
        writeln!(svg, r#"<rect x="{x}" y="100" width="12" height="12" fill="{background}" stroke="{border}"/><text x="{}" y="111" font-size="12" fill="{border}">{} ({count})</text>"#, x + 19, label(*action)).unwrap();
    }
    if graph.status != Default::default() {
        svg.push_str("<g id=\"plan-status\"");
        let flags = [
            ("applyable", graph.status.applyable),
            ("complete", graph.status.complete),
            ("errored", graph.status.errored),
        ];
        for (name, value) in flags {
            if let Some(value) = value {
                write!(svg, " data-plan-{name}=\"{value}\"").unwrap();
            }
        }
        svg.push_str(">\n<text x=\"40\" y=\"137\" font-size=\"12\" fill=\"#475569\">Plan status:");
        for (name, value) in flags {
            let value = match value {
                Some(true) => "true",
                Some(false) => "false",
                None => "unknown",
            };
            write!(svg, " {name}: {value}").unwrap();
        }
        svg.push_str("</text>\n</g>\n");
    }
    let legend_height = legend::height(graph);
    if legend_height != 0 {
        svg.push_str(&legend::render(graph));
        writeln!(svg, "<g transform=\"translate(0 {legend_height})\">").unwrap();
    }
    if check_offset != 0 {
        svg.push_str(&checks::render(&graph.checks));
        svg.push_str("<g transform=\"translate(0 24)\">\n");
    }
    for band in &layout.bands {
        let (module, y, h) = (band.label, band.top, band.height);
        writeln!(svg, r##"<rect x="35" y="{y}" width="{}" height="{h}" rx="12" fill="#f8fafc" stroke="#cbd5e1" stroke-dasharray="5 4"/><text x="52" y="{}" font-size="13" font-weight="700" fill="#475569"><title>{}</title>{}</text>"##, width - 70, y + 25, escape(module), escape(&shorten(module, 110))).unwrap();
    }
    svg.push_str(&containers::boundaries(graph, layout));
    for (edge, points) in graph.edges.iter().zip(&layout.paths) {
        let relation = match edge.kind {
            EdgeKind::Dependency => " data-edge-kind=\"dependency\"",
            EdgeKind::Association => " data-edge-kind=\"association\"",
            EdgeKind::Connection => " data-edge-kind=\"connection\"",
            EdgeKind::Containment => " data-edge-kind=\"containment\"",
        };
        let title = relationships::title(graph, edge, semantic_edges);
        let operation = edge
            .change
            .as_ref()
            .map(|change| operations::attributes(&change.metadata))
            .unwrap_or_default();
        let previous = edge
            .change
            .as_ref()
            .and_then(|change| change.previous_address.as_ref())
            .map(|address| format!(" data-previous-address=\"{}\"", escape(address)))
            .unwrap_or_default();
        let source = identity::resource(&graph.nodes[edge.from]);
        let target = identity::resource(&graph.nodes[edge.to]);
        if points.is_empty() {
            writeln!(svg, r#"<g{relation}{previous}{operation} data-source="{source}" data-target="{target}"><title>{title}</title></g>"#).unwrap();
            continue;
        }
        let (stroke, marker) = match &edge.change {
            Some(change) => (
                color(change.action).1,
                format!("arrow-{}", label(change.action)),
            ),
            None => ("#94a3b8", "arrow".into()),
        };
        let path = paths::rounded(points);
        let style = relationships::style(edge.kind);
        writeln!(svg, r##"<path d="{path}"{relation}{previous}{operation} data-source="{source}" data-target="{target}" fill="none" stroke="{stroke}" {style} stroke-linejoin="round" marker-end="url(#{marker})"><title>{title}</title></path>"##).unwrap();
    }
    for point in &layout.junctions {
        writeln!(
            svg,
            r##"<circle cx="{}" cy="{}" r="3" fill="#94a3b8"/>"##,
            point.x, point.y
        )
        .unwrap();
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        let resource_id = identity::resource(node);
        let architecture_id = escape(node.entity.id.as_str());
        let bounds = layout.bounds[i];
        let (x, y) = (bounds.origin.x, bounds.origin.y);
        let card_width = bounds.width;
        let header_height = layout.header_heights[i];
        let background = containers::background(graph, layout, i);
        let border = color(node.action).1;
        let resource_address = node.resource_address();
        let address = escape(resource_address.terraform());
        let qualified = escape(&resource_address.qualified());
        let selected = match address_format {
            AddressFormat::Qualified => resource_address.qualified(),
            AddressFormat::Terraform => resource_address.terraform().into(),
        };
        let selected_title = escape(&selected);
        let icon = icons::for_node(node);
        let inset = match icon {
            Some(_) => 12 + icons::SIZE + icons::GAP,
            None => 12,
        };
        let line_length = (card_width.saturating_sub(inset + 12) / 8).clamp(1, 35);
        let lines: Vec<_> = selected
            .chars()
            .collect::<Vec<_>>()
            .chunks(line_length)
            .take(2)
            .map(|c| c.iter().collect::<String>())
            .collect();
        let mode = entities::border(node.mode);
        let operation = operations::attributes(&node.metadata);
        let component = membership
            .get(node.address.as_str())
            .map(|id| format!(" data-component-id=\"{}\"", escape(id)))
            .unwrap_or_default();
        let operation_label = operations::label(node.action, &node.metadata);
        let operation_details = escape(&operations::details(&node.metadata));
        let previous = node
            .previous_address
            .as_ref()
            .map(|address| format!(" data-previous-address=\"{}\"", escape(address)))
            .unwrap_or_default();
        let deposed = node
            .deposed_key
            .as_ref()
            .map(|key| format!(" data-deposed-key=\"{}\"", escape(key)))
            .unwrap_or_default();
        writeln!(
            svg,
            r#"<g id="{resource_id}" data-architecture-id="{architecture_id}" data-qualified-address="{qualified}" data-terraform-address="{address}"{deposed}{previous}{operation}{component}><title>{selected_title} — {operation_label}{operation_details}</title>"#,
        )
        .unwrap();
        if node.role != crate::model::architecture::ResourceRole::Container {
            writeln!(svg, r#"<rect x="{x}" y="{y}" width="{card_width}" height="{header_height}" rx="8" fill="{background}" stroke="{border}" stroke-width="1.5"{mode}/>"#).unwrap();
        }
        svg.push_str(&entities::badge(node.mode, x, y));
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="11" fill="{border}">{} · {}</text>"#,
            x + 12,
            y + 20,
            escape(&shorten(
                &node.resource_type,
                entities::type_limit(node.mode).saturating_sub(
                    operation_label
                        .len()
                        .saturating_sub(label(node.action).len())
                )
            )),
            operation_label
        )
        .unwrap();
        if let Some(icon) = icon {
            svg.push_str(&icons::render(icon, x + 12, y + 28));
        }
        for (line, text) in lines.iter().enumerate() {
            let text = match line {
                1 if selected.chars().count() > line_length * 2 => {
                    format!(
                        "{}…",
                        text.chars().take(line_length - 1).collect::<String>()
                    )
                }
                _ => text.clone(),
            };
            writeln!(
                svg,
                r##"<text x="{}" y="{}" font-size="13" fill="#0f172a">{}</text>"##,
                x + inset,
                y + 45 + line * 20,
                escape(&text)
            )
            .unwrap();
        }
        let role = roles::annotation(node.role, &node.module);
        let footer = match role.is_empty() {
            false => role,
            true if !layout.containers.is_empty() => escape(&shorten(&node.module, 43)),
            true => String::new(),
        };
        if !footer.is_empty() {
            writeln!(svg, r##"<text x="{}" y="{}" font-size="10" fill="#64748b"{}><title>{}</title>{footer}</text>"##, x + 12, y + header_height - 12, roles::attributes(node.role), escape(&node.module)).unwrap();
        }
        svg.push_str("</g>\n");
    }
    if graph.nodes.is_empty() {
        svg.push_str("<text x=\"40\" y=\"185\" font-size=\"16\" fill=\"#64748b\">No resources to display.</text>\n");
    }
    if check_offset != 0 {
        svg.push_str("</g>\n");
    }
    svg.push_str(&components::render(graph, layout));
    if legend_height != 0 {
        svg.push_str("</g>\n");
    }
    svg.push_str("</g>\n</svg>\n");
    svg
}
