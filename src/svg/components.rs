use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
use crate::view::text::wrap_address;
use std::collections::BTreeMap;
use std::fmt::Write;

pub(super) fn membership(graph: &Graph) -> BTreeMap<&str, &str> {
    graph
        .components
        .iter()
        .filter(|component| component.kind != "organization_root")
        .flat_map(|component| {
            component
                .members
                .iter()
                .map(|member| (member.as_str(), component.id.as_str()))
        })
        .collect()
}

pub(super) fn cards(graph: &Graph, address_format: super::AddressFormat) -> Vec<Card> {
    let mut nodes = BTreeMap::<_, Vec<_>>::new();
    for node in &graph.nodes {
        nodes.entry(node.address.as_str()).or_default().push(node);
    }
    let mut components: Vec<_> = graph.components.iter().collect();
    components.sort_by_key(|component| &component.entity.id);
    components.into_iter().map(|component| {
        let dom_id = super::identity::architecture(&component.entity.id);
        let architecture_id = super::escape(component.entity.id.as_str());
        let heading = component.label_reference.as_ref().map_or_else(|| component.label.clone(), |reference| format!("{}{}{}", reference.prefix, reference.source.resource_address().formatted(matches!(address_format, super::AddressFormat::Qualified)), reference.suffix));
        let label = super::escape(&heading);
        let mut members: Vec<_> = component.members.iter().collect();
        members.sort();
        members.dedup();
        let rows: Vec<_> = members.iter().map(|address| {
            let node = nodes.get(address.as_str()).and_then(|nodes| {
                if component.kind == "organization_root" {
                    nodes.iter().copied().find(|node| node.entity.provenance.iter().any(|id| component.entity.provenance.contains(id)))
                } else {
                    match nodes.as_slice() {
                        [node] => Some(*node),
                        _ => None,
                    }
                }
            });
            let text = node.map_or_else(|| (*address).clone(), |node| node.resource_address().formatted(matches!(address_format, super::AddressFormat::Qualified)).into_owned());
            (address, node, text)
        }).collect();
        let mut scope = crate::view::scope::component(rows.iter().map(|(_, node, _)| *node));
        if component.kind == "organization_root" {
            scope = scope.replacen("Member scope:", "Source Organization scope:", 1);
        }
        let member_scopes: Vec<_> = rows.iter().map(|(_, node, _)| node.map_or_else(|| "Member unavailable in this view".into(), |node| crate::view::scope::label(node.entity.scope.as_ref(), &node.provider))).collect();
        let width = card_width([heading.as_str(), scope.as_str()].into_iter().chain(rows.iter().map(|(_, _, text)| text.as_str())).chain(member_scopes.iter().map(String::as_str)));
        let limit = (width - 32) / 8;
        let mut svg = format!(r#"<g id="{dom_id}" data-architecture-id="{architecture_id}" data-entity-kind="synthetic" data-component-id="{}" data-component-kind="{}"><title>{label}</title>"#, super::escape(&component.id), super::escape(&component.kind));
        let mut y = 28;
        for line in wrap(&heading, limit) {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="13" font-weight="700" fill="#5b21b6">{}</text>"##, super::escape(line)).unwrap();
            y += 20;
        }
        for line in wrap(&scope, limit) {
            writeln!(svg, r##"<text data-component-scope="true" x="16" y="{y}" font-size="12" fill="#64748b">{}</text>"##, super::escape(line)).unwrap();
            y += 20;
        }
        let member_width = width - 32;
        for ((address, node, text), member_scope) in rows.into_iter().zip(member_scopes) {
            let title = match node {
                Some(node) => format!("{}; {}{}{}{}", node.resource_address().formatted(matches!(address_format, super::AddressFormat::Qualified)), super::operations::label(node.action, &node.metadata), super::operations::details(&node.metadata), node.previous_address.as_ref().map(|address| format!("; moved from: {address}")).unwrap_or_default(), node.deposed_key.as_ref().map(|key| format!("; deposed: {key}")).unwrap_or_default()),
                None => (*address).clone(),
            };
            let lines = wrap_address(&text, member_width / 8);
            let badge_height = node.map_or(0, crate::view::card::import_height);
            let scope_lines = wrap(&member_scope, member_width / 8);
            let (action_field, action_height) = node.map(|node| super::operations::action_field(&super::operations::label(node.action, &node.metadata), 0, 20 + badge_height + lines.len() * 20, member_width / 8)).unwrap_or_default();
            let scope_y = lines.len() * 20 + action_height + 20 + badge_height;
            let underline_y = (lines.len() + scope_lines.len()) * 20 + action_height + 8 + badge_height;
            let (_, underline) = super::color(node.map_or(crate::model::Action::Unchanged, |node| node.action));
            let mut member = format!(r#"<g data-component-member="true" transform="translate(16 {y})">"#);
            if let Some(node) = node {
                write!(member, "<a href=\"#{}\" data-source-address=\"{}\"{}>", super::identity::resource(node), super::escape(address), super::operations::attributes(&node.metadata)).unwrap();
            } else {
                write!(member, "<g data-source-address=\"{}\" data-member-status=\"unavailable\">", super::escape(address)).unwrap();
            }
            write!(member, "<title>{}</title>", super::escape(&title)).unwrap();
            if let Some(node) = node {
                member.push_str(&super::entities::import_badge(node, 0, 0));
            }
            member.push_str("<text font-size=\"12\" fill=\"#334155\">\n");
            for (index, line) in lines.iter().enumerate() {
                writeln!(member, "<tspan x=\"0\" y=\"{}\" xml:space=\"preserve\">{}</tspan>", 20 + badge_height + index * 20, super::escape(line)).unwrap();
            }
            member.push_str("</text>");
            member.push_str(&action_field);
            for (index, line) in scope_lines.into_iter().enumerate() {
                writeln!(member, r##"<text data-member-scope="true" x="0" y="{}" font-size="12" fill="#64748b">{}</text>"##, scope_y + index * 20, super::escape(line)).unwrap();
            }
            writeln!(member, r#"<line data-member-underline="true" x1="0" y1="{underline_y}" x2="{member_width}" y2="{underline_y}" stroke="{underline}" stroke-width="2"/>"#).unwrap();
            member.push_str(if node.is_some() { "</a></g>\n" } else { "</g></g>\n" });
            svg.push_str(&member);
            y += underline_y + 16;
        }
        y += 16;
        svg.push_str("</g>\n");
        Card { width, height: y, content: svg }
    }).collect()
}

#[cfg(test)]
#[path = "../../tests/unit/svg/components.rs"]
mod tests;
