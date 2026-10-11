use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
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
    let nodes: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.address.as_str(), node))
        .collect();
    let mut components: Vec<_> = graph.components.iter().collect();
    components.sort_by_key(|component| &component.entity.id);
    components.into_iter().map(|component| {
        let dom_id = super::identity::architecture(&component.entity.id);
        let architecture_id = super::escape(component.entity.id.as_str());
        let label = super::escape(&component.label);
        let heading = format!("Logical component: {}", component.label);
        let mut members: Vec<_> = component.members.iter().collect();
        members.sort();
        members.dedup();
        let rows: Vec<_> = members.iter().map(|address| {
            let node = nodes.get(address.as_str()).copied();
            let text = node.map_or_else(|| (*address).clone(), |node| format!("{}: {address}", super::operations::label(node.action, &node.metadata)));
            (address, node, text)
        }).collect();
        let width = card_width(std::iter::once(heading.as_str()).chain(rows.iter().map(|(_, _, text)| text.as_str())));
        let limit = (width - 32) / 8;
        let mut svg = format!(r#"<g id="{dom_id}" data-architecture-id="{architecture_id}" data-entity-kind="synthetic" data-component-id="{}" data-component-kind="{}"><title>{label}</title>"#, super::escape(&component.id), super::escape(&component.kind));
        let mut y = 28;
        for line in wrap(&heading, limit) {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="13" font-weight="700" fill="#5b21b6">{}</text>"##, super::escape(line)).unwrap();
            y += 20;
        }
        writeln!(svg, r##"<text x="16" y="{y}" font-size="12" font-weight="700" fill="#475569">Members</text>"##).unwrap();
        y += 24;
        for (address, node, text) in rows {
            let title = match node {
                Some(node) => format!("{address}; {}{}", super::operations::label(node.action, &node.metadata), super::operations::details(&node.metadata)),
                None => (*address).clone(),
            };
            if let Some(node) = node {
                write!(svg, "<a href=\"#{}\" data-source-address=\"{}\">", super::identity::resource(node), super::escape(address)).unwrap();
            } else {
                write!(svg, "<g data-source-address=\"{}\">", super::escape(address)).unwrap();
            }
            writeln!(svg, r##"<title>{}</title><text font-size="12" fill="#334155">"##, super::escape(&title)).unwrap();
            for line in wrap(&text, limit) {
                writeln!(svg, "<tspan x=\"16\" y=\"{y}\" xml:space=\"preserve\">{}</tspan>", super::escape(line)).unwrap();
                y += 20;
            }
            svg.push_str(if node.is_some() { "</text></a>\n" } else { "</text></g>\n" });
            y += 4;
        }
        svg.push_str("</g>\n");
        Card { width, height: y, content: svg }
    }).collect()
}

#[cfg(test)]
#[path = "../../tests/unit/svg/components.rs"]
mod tests;
