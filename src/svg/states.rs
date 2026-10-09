use crate::layout::Layout;
use crate::model::{ArchitectureGraph, StateId};
use std::collections::BTreeMap;
use std::fmt::Write;

pub fn render_states(
    graphs: &BTreeMap<StateId, ArchitectureGraph>,
    format: super::AddressFormat,
) -> String {
    let mut sections = String::new();
    let mut top = 60;
    let mut width = 1120;
    for (id, graph) in graphs {
        let layout = Layout::new(graph);
        width = width.max(layout.width);
        let (_, height) = super::dimensions(graph, &layout);
        let prefix = prefix(id);
        let document = namespace(&super::render_with_format(graph, &layout, format), &prefix);
        writeln!(sections, r##"<g id="{prefix}state" data-state-id="{}"><text x="40" y="{}" font-family="ui-monospace, Consolas, monospace" font-size="20" font-weight="700" fill="#0f172a">State: {}</text>"##, super::escape(id.as_str()), top + 25, super::escape(id.as_str())).unwrap();
        sections.push_str(&document.replacen("<svg ", &format!("<svg y=\"{}\" ", top + 40), 1));
        sections.push_str("</g>\n");
        top += height + 60;
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
