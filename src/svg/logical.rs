use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
use std::fmt::Write;

pub(super) fn cards(graph: &Graph) -> Vec<Card> {
    let mut nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.placement == crate::model::Placement::Logical)
        .collect();
    nodes.sort_by_key(|n| &n.entity.id);
    nodes.into_iter().map(|node| {
        let mode = match node.mode { crate::model::EntityMode::Data => "data / external", crate::model::EntityMode::Managed => "managed" };
        let operation = super::operations::label(node.action, &node.metadata);
        let mut lines = vec!["Logical resource".to_string(), node.address.clone(), format!("{} · {mode}", node.resource_type), operation.clone()];
        if let Some(previous) = &node.previous_address { lines.push(format!("Moved from: {previous}")); }
        if let Some(key) = &node.deposed_key { lines.push(format!("Deposed: {key}")); }
        let width = card_width(lines.iter());
        let mut svg = format!(r#"<g id="{}" data-architecture-id="{}" data-entity-kind="terraform" data-placement="logical" data-terraform-address="{}" data-qualified-address="{}"{}><title>{} — {}{}</title>"#,
            super::identity::resource(node), super::escape(node.entity.id.as_str()), super::escape(&node.address), super::escape(&node.resource_address().qualified()), super::operations::attributes(&node.metadata), super::escape(&node.address), super::escape(&operation), super::escape(&super::operations::details(&node.metadata)));
        for (name, value) in [("previous-address", &node.previous_address), ("deposed-key", &node.deposed_key)] {
            if let Some(value) = value {
                let close = svg.find('>').unwrap();
                svg.insert_str(close, &format!(" data-{name}=\"{}\"", super::escape(value)));
            }
        }
        if node.mode == crate::model::EntityMode::Data {
            svg.insert_str(svg.find('>').unwrap(), " data-external=\"true\"");
        }
        let mut y = 28;
        let (background, border) = super::color(node.action);
        let mode_attributes = super::entities::border(node.mode);
        let reference_height = crate::view::references::height(node);
        let height = lines.iter().map(|line| wrap(line, (width - 32) / 8).len()).sum::<usize>() * 20 + 28 + reference_height;
        writeln!(svg, r#"<rect width="{width}" height="{height}" rx="12" fill="{background}" stroke="{border}"{mode_attributes}/>"#).unwrap();
        if let Some(icon) = super::icons::for_node(node) {
            svg.push_str(&super::icons::render(icon, width - 40, 8));
        }
        for (index, text) in lines.iter().enumerate() {
            for line in wrap(text, (width - 32) / 8) {
                writeln!(svg, r##"<text x="16" y="{y}" font-size="12" fill="{}"{}>{}</text>"##, if index == 0 { "#5b21b6" } else { "#334155" }, if index == 0 { " font-weight=\"700\"" } else { "" }, super::escape(line)).unwrap();
                y += 20;
            }
        }
        svg.push_str(&super::references::render(graph, node, 16, y + 8, width - 32));
        y += reference_height;
        svg.push_str("</g>\n");
        Card { width, height: y, content: svg }
    }).collect()
}

#[cfg(test)]
#[path = "../../tests/unit/svg/logical.rs"]
mod tests;
