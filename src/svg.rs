mod containers;
mod entities;
mod paths;
mod relationships;
mod roles;
mod style;
#[cfg(test)]
#[path = "../tests/unit/svg.rs"]
mod tests;

use crate::layout::Layout;
use crate::model::{Action, EdgeKind, Graph};
use std::fmt::Write;
use style::{color, label, shorten};

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

pub fn render(graph: &Graph, layout: &Layout<'_>) -> String {
    let (width, height) = (layout.width, layout.height);
    let (description, summary) = relationships::captions(graph);
    let markers = relationships::markers(graph);
    let semantic_edges = graph
        .edges
        .iter()
        .any(|edge| edge.kind != EdgeKind::Dependency);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">
<title id="title">Terraform plan</title>
<desc id="description">{description}</desc>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8"/></marker>{markers}</defs>
<rect width="100%" height="100%" fill="#ffffff"/>
<g font-family="ui-monospace, SFMono-Regular, Consolas, monospace">
<text x="40" y="45" font-size="25" font-weight="700" fill="#0f172a">Terraform plan</text>
<text x="40" y="73" font-size="13" fill="#475569">{summary}</text>
"##,
    );
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
    for band in &layout.bands {
        let (module, y, h) = (band.label, band.top, band.height);
        writeln!(svg, r##"<rect x="35" y="{y}" width="{}" height="{h}" rx="12" fill="#f8fafc" stroke="#cbd5e1" stroke-dasharray="5 4"/><text x="52" y="{}" font-size="13" font-weight="700" fill="#475569"><title>{}</title>{}</text>"##, width - 70, y + 25, escape(module), escape(&shorten(module, 110))).unwrap();
    }
    svg.push_str(&containers::boundaries(graph, layout));
    for (edge, points) in graph.edges.iter().zip(&layout.paths) {
        let relation = match edge.kind {
            EdgeKind::Dependency => "",
            EdgeKind::Association => " data-edge-kind=\"association\"",
            EdgeKind::Connection => " data-edge-kind=\"connection\"",
            EdgeKind::Containment => " data-edge-kind=\"containment\"",
        };
        let title = relationships::title(graph, edge, semantic_edges);
        if points.is_empty() {
            writeln!(svg, r#"<g{relation} data-source="resource-{}" data-target="resource-{}"><title>{title}</title></g>"#, edge.from, edge.to).unwrap();
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
        writeln!(svg, r##"<path d="{path}"{relation} fill="none" stroke="{stroke}" stroke-width="1.5" stroke-linejoin="round" marker-end="url(#{marker})"><title>{title}</title></path>"##).unwrap();
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
        let bounds = layout.bounds[i];
        let (x, y) = (bounds.origin.x, bounds.origin.y);
        let card_width = bounds.width;
        let header_height = layout.header_heights[i];
        let background = containers::background(graph, layout, i);
        let border = color(node.action).1;
        let resource_address = node.resource_address();
        let address = escape(resource_address.terraform());
        let qualified = escape(&resource_address.qualified());
        let local = resource_address.local();
        let lines: Vec<_> = local
            .chars()
            .collect::<Vec<_>>()
            .chunks(35)
            .take(2)
            .map(|c| c.iter().collect::<String>())
            .collect();
        let mode = entities::border(node.mode);
        writeln!(
            svg,
            r#"<g id="resource-{i}" data-qualified-address="{qualified}"><title>{address} — {}</title>"#,
            label(node.action)
        )
        .unwrap();
        if node.role != crate::model::ResourceRole::Container {
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
                entities::type_limit(node.mode)
            )),
            label(node.action)
        )
        .unwrap();
        for (line, text) in lines.iter().enumerate() {
            let text = match line {
                1 if local.chars().count() > 70 => {
                    format!("{}…", text.chars().take(34).collect::<String>())
                }
                _ => text.clone(),
            };
            writeln!(
                svg,
                r##"<text x="{}" y="{}" font-size="13" fill="#0f172a">{}</text>"##,
                x + 12,
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
    svg.push_str("</g>\n</svg>\n");
    svg
}
