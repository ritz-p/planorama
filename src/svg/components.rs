use crate::model::architecture::Graph;
use std::collections::BTreeMap;
use std::fmt::Write;

pub(super) fn height(graph: &Graph) -> usize {
    graph
        .components
        .iter()
        .map(|component| 76 + component.members.len() * 24)
        .sum()
}

pub(super) fn membership(graph: &Graph) -> BTreeMap<&str, &str> {
    graph
        .components
        .iter()
        .flat_map(|component| {
            component
                .members
                .iter()
                .map(|member| (member.as_str(), component.id.as_str()))
        })
        .collect()
}

pub(super) fn render(graph: &Graph, layout: &crate::layout::Layout<'_>) -> String {
    let width = layout.width;
    let nodes: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.address.as_str(), (i, n)))
        .collect();
    let mut svg = String::new();
    for (component, (id, bounds)) in graph.components.iter().zip(layout.component_bounds(graph)) {
        let height = bounds.height;
        let top = bounds.origin.y;
        let dom_id = super::identity::architecture(id);
        let architecture_id = super::escape(id.as_str());
        let label = super::escape(&component.label);
        let limit = width.saturating_sub(120) / 8;
        let heading = super::escape(&super::style::shorten(
            &format!("Logical component: {}", component.label),
            limit,
        ));
        writeln!(svg, r##"<g id="{dom_id}" data-architecture-id="{architecture_id}" data-entity-kind="synthetic" data-component-id="{}" data-component-kind="{}"><title>{label}</title><rect x="35" y="{top}" width="{}" height="{height}" rx="12" fill="#f5f3ff" stroke="#8b5cf6" stroke-dasharray="6 3"/><text x="52" y="{}" font-size="13" font-weight="700" fill="#5b21b6">{heading}</text>"##, super::escape(&component.id), super::escape(&component.kind), width - 70, top + 24).unwrap();
        for (row, address) in component.members.iter().enumerate() {
            if let Some((_, node)) = nodes.get(address.as_str()) {
                let resource_id = super::identity::resource(node);
                let operation = super::operations::label(node.action, &node.metadata);
                let title = super::escape(&format!(
                    "{address}; {operation}{}",
                    super::operations::details(&node.metadata)
                ));
                let text = super::escape(&super::style::shorten(
                    &format!("{operation}: {address}"),
                    limit,
                ));
                writeln!(svg, r##"<a href="#{resource_id}" data-source-address="{}"><title>{title}</title><text x="52" y="{}" font-size="12" fill="#334155">{text}</text></a>"##, super::escape(address), top + 48 + row * 24).unwrap();
            }
        }
        svg.push_str("</g>\n");
    }
    svg
}
