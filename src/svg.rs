//! Serialize the graph and its geometry as a standalone SVG document.
mod style;
#[cfg(test)]
mod tests;

use crate::layout::Layout;
use crate::model::{Action, Graph};
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
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">
<title id="title">Terraform plan</title>
<desc id="description">{} resources, {} reference edges. Arrows point from dependencies to dependent resources. References are conservative, not Terraform execution order.</desc>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8"/></marker></defs>
<rect width="100%" height="100%" fill="#ffffff"/>
<g font-family="ui-monospace, SFMono-Regular, Consolas, monospace">
<text x="40" y="45" font-size="25" font-weight="700" fill="#0f172a">Terraform plan</text>
<text x="40" y="73" font-size="13" fill="#475569">{} resources · {} reference edges · dependency → dependent</text>
"##,
        graph.nodes.len(),
        graph.edges.len(),
        graph.nodes.len(),
        graph.edges.len()
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
        let count = graph.nodes.iter().filter(|n| n.action == *action).count();
        writeln!(svg, r#"<rect x="{x}" y="100" width="12" height="12" fill="{background}" stroke="{border}"/><text x="{}" y="111" font-size="12" fill="{border}">{} ({count})</text>"#, x + 19, label(*action)).unwrap();
    }
    for band in &layout.bands {
        let (module, y, h) = (band.label, band.top, band.height);
        writeln!(svg, r##"<rect x="35" y="{y}" width="{}" height="{h}" rx="12" fill="#f8fafc" stroke="#cbd5e1" stroke-dasharray="5 4"/><text x="52" y="{}" font-size="13" font-weight="700" fill="#475569"><title>{}</title>{}</text>"##, width - 70, y + 25, escape(module), escape(&shorten(module, 110))).unwrap();
    }
    for (&(a, b), points) in graph.edges.iter().zip(&layout.paths) {
        let title = escape(&format!(
            "{} → {}",
            graph.nodes[a].address, graph.nodes[b].address
        ));
        let mut path = String::new();
        for (i, point) in points.iter().enumerate() {
            write!(
                path,
                "{} {} {} ",
                match i {
                    0 => "M",
                    _ => "L",
                },
                point.x,
                point.y
            )
            .unwrap();
        }
        writeln!(svg, r##"<path d="{path}" fill="none" stroke="#94a3b8" stroke-width="1.5" stroke-linejoin="round" marker-end="url(#arrow)"><title>{title}</title></path>"##).unwrap();
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        let (x, y) = (layout.positions[i].x, layout.positions[i].y);
        let (background, border) = color(node.action);
        let address = escape(&node.address);
        let local = match node.module.as_str() {
            "root" => node.address.as_str(),
            module => node
                .address
                .strip_prefix(&format!("{module}."))
                .unwrap_or(&node.address),
        };
        let lines: Vec<_> = local
            .chars()
            .collect::<Vec<_>>()
            .chunks(35)
            .take(2)
            .map(|c| c.iter().collect::<String>())
            .collect();
        writeln!(svg, r#"<g id="resource-{i}"><title>{address} — {}</title><rect x="{x}" y="{y}" width="320" height="96" rx="8" fill="{background}" stroke="{border}" stroke-width="1.5"/>"#, label(node.action)).unwrap();
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="11" fill="{border}">{} · {}</text>"#,
            x + 12,
            y + 20,
            escape(&shorten(&node.resource_type, 25)),
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
        svg.push_str("</g>\n");
    }
    if graph.nodes.is_empty() {
        svg.push_str("<text x=\"40\" y=\"185\" font-size=\"16\" fill=\"#64748b\">No resources in this plan.</text>\n");
    }
    svg.push_str("</g>\n</svg>\n");
    svg
}
