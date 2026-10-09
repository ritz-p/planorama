use crate::layout::Layout;
use crate::model::{
    StateId,
    cross_state::{Architecture, Endpoint},
};
use std::collections::BTreeMap;
use std::fmt::Write;

pub fn render_states(architecture: &Architecture, format: super::AddressFormat) -> String {
    let graphs = &architecture.states;
    let mut endpoints = BTreeMap::new();
    let mut sections = String::new();
    let mut top = 60;
    let mut width = 1120;
    for (id, graph) in graphs {
        let layout = Layout::new(graph);
        width = width.max(layout.width);
        let (_, height) = super::dimensions(graph, &layout);
        let prefix = prefix(id);
        let check_offset = if graph.checks.is_empty() { 0 } else { 24 };
        for (index, node) in graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.deposed_key.is_none())
        {
            let bounds = layout.bounds[index];
            endpoints.insert(
                Endpoint {
                    state: id.clone(),
                    address: node.address.clone(),
                },
                (
                    bounds.right(),
                    top + 40 + check_offset + bounds.origin.y + layout.header_heights[index] / 2,
                    format!("{prefix}resource-{index}"),
                ),
            );
        }
        let document = namespace(&super::render_with_format(graph, &layout, format), &prefix);
        writeln!(sections, r##"<g id="{prefix}state" data-state-id="{}"><text x="40" y="{}" font-family="ui-monospace, Consolas, monospace" font-size="20" font-weight="700" fill="#0f172a">State: {}</text>"##, super::escape(id.as_str()), top + 25, super::escape(id.as_str())).unwrap();
        sections.push_str(&document.replacen("<svg ", &format!("<svg y=\"{}\" ", top + 40), 1));
        sections.push_str("</g>\n");
        top += height + 60;
    }
    let mut links = String::new();
    for edge in &architecture.relationships {
        let (Some((sx, sy, source)), Some((tx, ty, target))) =
            (endpoints.get(&edge.from), endpoints.get(&edge.to))
        else {
            continue;
        };
        let lane = width + 30;
        let title = super::escape(&format!(
            "{}:{} -> {}:{} via {}.outputs.{} (cross-state dependency)",
            edge.from.state.as_str(),
            edge.from.address,
            edge.to.state.as_str(),
            edge.to.address,
            edge.remote,
            edge.output
        ));
        writeln!(links, r##"<path data-edge-kind="dependency" data-cross-state="true" data-source="{source}" data-target="{target}" data-source-state="{}" data-target-state="{}" data-remote-state="{}" data-output="{}" d="M {sx} {sy} H {lane} V {ty} H {tx}" fill="none" stroke="#2563eb" stroke-width="1.5" stroke-dasharray="7 4" marker-end="url(#cross-state-arrow)"><title>{title}</title></path>"##, super::escape(edge.from.state.as_str()),super::escape(edge.to.state.as_str()),super::escape(&edge.remote),super::escape(&edge.output)).unwrap();
    }
    if !links.is_empty() {
        width += 80;
        sections.push_str("<defs><marker id=\"cross-state-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#2563eb\"/></marker></defs>\n");
        sections.push_str(&links);
        sections.push_str("<text x=\"350\" y=\"35\" font-family=\"ui-monospace, Consolas, monospace\" font-size=\"12\" fill=\"#2563eb\">Dashed blue: cross-state dependency</text>\n");
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{top}" viewBox="0 0 {width} {top}" role="img" aria-labelledby="multi-title multi-description">
<title id="multi-title">Terraform plans</title><desc id="multi-description">{} independent states. Resource identities and relationships are scoped to each state.</desc>
<rect width="100%" height="100%" fill="#ffffff"/><text x="40" y="35" font-family="ui-monospace, Consolas, monospace" font-size="25" fill="#0f172a">Terraform plans</text>
{sections}</svg>
"##,
        graphs.len()
    )
}

// Hex encoding is injective and independent of file paths and input ordering.
fn prefix(id: &StateId) -> String {
    let mut prefix = String::from("state-");
    for byte in id.as_str().bytes() {
        write!(prefix, "{byte:02x}").unwrap();
    }
    prefix.push('-');
    prefix
}

fn namespace(svg: &str, prefix: &str) -> String {
    // These are renderer-owned quoted attributes. User content is XML-escaped
    // before this stage, so it cannot masquerade as an attribute or reference.
    svg.replace(" id=\"", &format!(" id=\"{prefix}"))
        .replace(" href=\"#", &format!(" href=\"#{prefix}"))
        .replace("marker-end=\"url(#", &format!("marker-end=\"url(#{prefix}"))
        .replace(
            "data-source=\"resource-",
            &format!("data-source=\"{prefix}resource-"),
        )
        .replace(
            "data-target=\"resource-",
            &format!("data-target=\"{prefix}resource-"),
        )
        .replace(
            "aria-labelledby=\"title description\"",
            &format!("aria-labelledby=\"{prefix}title {prefix}description\""),
        )
}
