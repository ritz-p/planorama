use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
use crate::view::text::wrap_address;
use std::collections::BTreeMap;
use std::fmt::Write;

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

pub(super) fn cards(graph: &Graph) -> Vec<Card> {
    let mut nodes = BTreeMap::<_, Vec<_>>::new();
    for node in &graph.nodes {
        nodes.entry(node.address.as_str()).or_default().push(node);
    }
    let mut components: Vec<_> = graph.components.iter().collect();
    components.sort_by_key(|component| &component.entity.id);
    components.into_iter().map(|component| {
        let dom_id = super::identity::architecture(&component.entity.id);
        let architecture_id = super::escape(component.entity.id.as_str());
        let label = super::escape(&component.label);
        let heading = &component.label;
        let mut members: Vec<_> = component.members.iter().collect();
        members.sort();
        members.dedup();
        let rows: Vec<_> = members.iter().map(|address| {
            let node = nodes.get(address.as_str()).and_then(|nodes| match nodes.as_slice() {
                [node] => Some(*node),
                _ => None,
            });
            let text = node.map_or_else(|| (*address).clone(), |node| format!("{}: {address}", super::operations::label(node.action, &node.metadata)));
            (address, node, text)
        }).collect();
        let width = card_width(std::iter::once(heading.as_str()).chain(rows.iter().map(|(_, _, text)| text.as_str())));
        let limit = (width - 32) / 8;
        let mut svg = format!(r#"<g id="{dom_id}" data-architecture-id="{architecture_id}" data-entity-kind="synthetic" data-component-id="{}" data-component-kind="{}"><title>{label}</title>"#, super::escape(&component.id), super::escape(&component.kind));
        let mut y = 28;
        for line in wrap(heading, limit) {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="13" font-weight="700" fill="#5b21b6">{}</text>"##, super::escape(line)).unwrap();
            y += 20;
        }
        let mut member_cards = Vec::new();
        for (address, node, text) in rows {
            let title = match node {
                Some(node) => format!("{}; {}{}{}{}", node.resource_address().qualified(), super::operations::label(node.action, &node.metadata), super::operations::details(&node.metadata), node.previous_address.as_ref().map(|address| format!("; moved from: {address}")).unwrap_or_default(), node.deposed_key.as_ref().map(|key| format!("; deposed: {key}")).unwrap_or_default()),
                None => (*address).clone(),
            };
            let member_width = card_width(std::iter::once(&text)).min(width - 32);
            let lines = wrap_address(&text, (member_width - 32) / 8);
            let height = lines.len() * 20 + 24;
            let (background, border) = super::color(node.map_or(crate::model::Action::Unchanged, |node| node.action));
            let mut member = String::new();
            if let Some(node) = node {
                write!(member, "<a href=\"#{}\" data-source-address=\"{}\"{}>", super::identity::resource(node), super::escape(address), super::operations::attributes(&node.metadata)).unwrap();
            } else {
                write!(member, "<g data-source-address=\"{}\" data-member-status=\"unavailable\">", super::escape(address)).unwrap();
            }
            writeln!(member, r##"<title>{}</title><rect width="{member_width}" height="{height}" rx="8" fill="{background}" stroke="{border}"/><text font-size="12" fill="#334155">"##, super::escape(&title)).unwrap();
            for (index, line) in lines.into_iter().enumerate() {
                writeln!(member, "<tspan x=\"16\" y=\"{}\" xml:space=\"preserve\">{}</tspan>", 28 + index * 20, super::escape(line)).unwrap();
            }
            member.push_str(if node.is_some() { "</text></a>\n" } else { "</text></g>\n" });
            member_cards.push(Card { width: member_width, height, content: member });
        }
        let members = super::annotations::Area::flow(member_cards, width, crate::layout::Point { x: 16, y }, 16, 16);
        svg.push_str(&members.content());
        y += members.height;
        svg.push_str("</g>\n");
        Card { width, height: y, content: svg }
    }).collect()
}

#[cfg(test)]
#[path = "../../tests/unit/svg/components.rs"]
mod tests;
