use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
use crate::view::text::wrap_address;
use std::fmt::Write;

pub(super) fn cards(graph: &Graph, address_format: super::AddressFormat) -> Vec<Card> {
    let mut nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.placement == crate::model::Placement::Logical)
        .collect();
    nodes.sort_by_key(|n| &n.entity.id);
    nodes.into_iter().map(|node| {
        let mode = match node.mode { crate::model::EntityMode::Data => "data / external", crate::model::EntityMode::Managed => "managed" };
        let operation = super::operations::label(node.action, &node.metadata);
        let address = match address_format {
            super::AddressFormat::Qualified => node.resource_address().qualified().into_owned(),
            super::AddressFormat::Terraform => node.address.clone(),
        };
        let mut lines = vec![(address.clone(), true), (format!("{} · {mode}", node.resource_type), false), (super::operations::action_label(node.action, &node.metadata).into(), false)];
        match node.role {
            crate::model::ResourceRole::Policy => lines.push(("policy".into(), false)),
            crate::model::ResourceRole::Controller => lines.push(("controller".into(), false)),
            _ => {}
        }
        if let Some(previous) = &node.previous_address { lines.push((crate::view::text::moved_from(previous), true)); }
        if let Some(key) = &node.deposed_key { lines.push((format!("Deposed: {key}"), false)); }
        let width = card_width(lines.iter().map(|(text, _)| text)).max(crate::view::references::width(node));
        let mut svg = format!(r#"<g id="{}" data-architecture-id="{}" data-entity-kind="terraform" data-placement="logical" data-terraform-address="{}" data-qualified-address="{}"{}><title>{} — {}{}{}</title>"#,
            super::identity::resource(node), super::escape(node.entity.id.as_str()), super::escape(&node.address), super::escape(&node.resource_address().qualified()), super::operations::attributes(&node.metadata), super::escape(&address), super::escape(&operation), super::escape(&super::operations::details(&node.metadata)), super::escape(&super::operations::move_details(node.previous_address.as_deref())));
        svg.insert_str(svg.find('>').unwrap(), super::roles::attributes(node.role));
        for (name, value) in [("previous-address", &node.previous_address), ("deposed-key", &node.deposed_key)] {
            if let Some(value) = value {
                let close = svg.find('>').unwrap();
                svg.insert_str(close, &format!(" data-{name}=\"{}\"", super::escape(value)));
            }
        }
        if node.mode == crate::model::EntityMode::Data {
            svg.insert_str(svg.find('>').unwrap(), " data-external=\"true\"");
        }
        let badge_height = crate::view::card::import_height(node);
        let mut y = 28 + badge_height;
        let (background, border) = super::color(node.action);
        let mode_attributes = super::entities::border(node.mode);
        let reference_height = crate::view::references::height(node);
        let line_limit = |index| (width - if index == 0 { 64 } else { 32 }) / 8;
        let wrapped: Vec<_> = lines.iter().enumerate().map(|(index, (line, is_address))| {
            if *is_address { wrap_address(line, line_limit(index)) } else { wrap(line, line_limit(index)) }
        }).collect();
        let height = wrapped.iter().map(Vec::len).sum::<usize>() * 20 + 28 + reference_height + badge_height;
        writeln!(svg, r#"<rect width="{width}" height="{height}" rx="12" fill="{background}" stroke="{border}"{mode_attributes}/>"#).unwrap();
        svg.push_str(&super::entities::import_badge(node, 16, 10));
        if let Some(icon) = super::icons::for_node(node) {
            svg.push_str(&super::icons::render(icon, width - 40, 8));
        }
        for (index, text) in wrapped.into_iter().enumerate() {
            for line in text {
                writeln!(svg, r##"<text x="16" y="{y}" font-size="12" fill="#334155" xml:space="preserve"{}>{}</text>"##, if index == 0 { " font-weight=\"700\"" } else { "" }, super::escape(line)).unwrap();
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
