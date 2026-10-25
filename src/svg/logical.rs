use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::{Graph, Node};
use crate::model::{Action, EntityMode, Placement};
use crate::view::text::wrap_address;
use std::collections::BTreeMap;
use std::fmt::Write;

type GroupKey<'a> = (Option<&'a str>, &'a str, &'static str);

fn mode(node: &Node) -> &'static str {
    match node.mode {
        EntityMode::Data => "data",
        EntityMode::Managed => "managed",
    }
}

fn groups(graph: &Graph) -> BTreeMap<GroupKey<'_>, Vec<&Node>> {
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.placement == Placement::Logical)
    {
        groups
            .entry((
                node.provider.source(),
                node.resource_type.as_str(),
                mode(node),
            ))
            .or_default()
            .push(node);
    }
    groups
}

pub(super) fn group_count(graph: &Graph) -> usize {
    groups(graph).len()
}

fn resource_lines(node: &Node, address_format: super::AddressFormat) -> Vec<(String, bool)> {
    let address = match address_format {
        super::AddressFormat::Qualified => node.resource_address().qualified().into_owned(),
        super::AddressFormat::Terraform => node.address.clone(),
    };
    let mut lines = vec![
        (address, true),
        (
            super::operations::action_label(node.action, &node.metadata).into(),
            false,
        ),
    ];
    lines.push((
        crate::view::scope::label(node.entity.scope.as_ref(), &node.provider),
        false,
    ));
    match node.role {
        crate::model::ResourceRole::Identity => lines.push(("identity".into(), false)),
        crate::model::ResourceRole::Policy => lines.push(("policy".into(), false)),
        crate::model::ResourceRole::Controller => lines.push(("controller".into(), false)),
        _ => {}
    }
    if let Some(previous) = &node.previous_address {
        lines.push((crate::view::text::moved_from(previous), true));
    }
    if let Some(key) = &node.deposed_key {
        lines.push((format!("Deposed: {key}"), false));
    }
    lines
}

fn resource_row(
    graph: &Graph,
    node: &Node,
    lines: &[(String, bool)],
    width: usize,
    origin_y: usize,
) -> (String, usize) {
    let operation = super::operations::label(node.action, &node.metadata);
    let mut svg = format!(
        r#"<g id="{}" data-architecture-id="{}" data-entity-kind="terraform" data-placement="logical" data-terraform-address="{}" data-qualified-address="{}" data-mode="{}" transform="translate(0 {origin_y})"{}><title>{} — {}{}{}{}</title>"#,
        super::identity::resource(node),
        super::escape(node.entity.id.as_str()),
        super::escape(&node.address),
        super::escape(&node.resource_address().qualified()),
        mode(node),
        super::operations::attributes(&node.metadata),
        super::escape(&lines[0].0),
        super::escape(&operation),
        super::escape(&super::operations::details(&node.metadata)),
        super::escape(&super::operations::move_details(
            node.previous_address.as_deref()
        )),
        super::escape(
            &node
                .deposed_key
                .as_ref()
                .map(|key| format!("; deposed: {key}"))
                .unwrap_or_default()
        )
    );
    svg.insert_str(svg.find('>').unwrap(), super::roles::attributes(node.role));
    for (name, value) in [
        ("previous-address", &node.previous_address),
        ("deposed-key", &node.deposed_key),
    ] {
        if let Some(value) = value {
            let close = svg.find('>').unwrap();
            svg.insert_str(close, &format!(" data-{name}=\"{}\"", super::escape(value)));
        }
    }
    if node.mode == EntityMode::Data {
        svg.insert_str(svg.find('>').unwrap(), " data-external=\"true\"");
    }
    let mut y = 28 + crate::view::card::import_height(node);
    svg.push_str(&super::entities::import_badge(node, 16, 10));
    if let Some(icon) = super::icons::for_node(node) {
        svg.push_str(&super::icons::render(icon, width - 40, 8));
    }
    if let Some(name) = crate::services::display_name(node) {
        let service_width = crate::view::service_label::width(name, width - 32);
        svg.push_str(&super::icons::service_name(
            node,
            width - 16,
            y - 16,
            service_width,
        ));
        y += crate::view::service_label::height(name, service_width);
    }
    for (index, (text, is_address)) in lines.iter().enumerate() {
        let limit = (width
            - if index == 0 && node.service_name.is_none() {
                64
            } else {
                32
            })
            / 8;
        let wrapped = if *is_address {
            wrap_address(text, limit)
        } else {
            wrap(text, limit)
        };
        for line in wrapped {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="12" fill="#334155" xml:space="preserve"{}>{}</text>"##, if index == 0 { " font-weight=\"700\"" } else { "" }, super::escape(line)).unwrap();
            y += 20;
        }
    }
    let reference_height = crate::view::references::height(node);
    svg.push_str(&super::references::render(
        graph,
        node,
        16,
        y + 8,
        width - 32,
    ));
    let underline_y = y + reference_height - if reference_height == 0 { 12 } else { 0 };
    let underline_end = width - 16;
    let (_, stroke) = super::color(node.action);
    let dash = if node.mode == EntityMode::Data {
        " stroke-dasharray=\"6 4\""
    } else {
        ""
    };
    writeln!(svg, r#"<line data-resource-underline="true" x1="16" y1="{underline_y}" x2="{underline_end}" y2="{underline_y}" stroke="{stroke}" stroke-width="2"{dash}/></g>"#).unwrap();
    (svg, underline_y + 16)
}

pub(super) fn cards(graph: &Graph, address_format: super::AddressFormat) -> Vec<Card> {
    groups(graph).into_iter().map(|((provider, resource_type, mode), mut nodes)| {
        nodes.sort_by_key(|node| &node.entity.id);
        let heading = format!("{resource_type} · {}", if mode == "data" { "data / external" } else { mode });
        let provider_label = provider.unwrap_or("Provider unavailable");
        let explanations: std::collections::BTreeSet<_> = nodes.iter().filter_map(|node| super::roles::explanation(node.role)).collect();
        let rows: Vec<_> = nodes.into_iter().map(|node| (node, resource_lines(node, address_format))).collect();
        let heading_width = card_width([heading.as_str(), provider_label].into_iter());
        let width = rows.iter().fold(heading_width, |width, (node, lines)| {
            width.max(card_width(lines.iter().map(|(text, _)| text))).max(crate::view::references::width(node))
        });
        let mut svg = format!(r#"<g data-logical-group="true" data-provider-source="{}" data-resource-type="{}" data-mode="{mode}"><title>{} — {}</title>"#, super::escape(provider.unwrap_or("unknown")), super::escape(resource_type), super::escape(&heading), super::escape(provider_label));
        let mut y = 28;
        for line in wrap(&heading, (width - 32) / 8) {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="13" font-weight="700" fill="{}">{}</text>"##, super::style::LOGICAL_RESOURCES.heading, super::escape(line)).unwrap();
            y += 20;
        }
        for line in wrap(provider_label, (width - 32) / 8) {
            writeln!(svg, r##"<text x="16" y="{y}" font-size="11" fill="#64748b">{}</text>"##, super::escape(line)).unwrap();
            y += 18;
        }
        for explanation in explanations {
            for line in wrap(explanation, (width - 32) / 8) {
                writeln!(svg, r##"<text data-role-explanation="true" x="16" y="{y}" font-size="12" fill="#475569" xml:space="preserve">{}</text>"##, super::escape(line)).unwrap();
                y += 20;
            }
        }
        for (action, label) in [
            (Action::Create, "Create"), (Action::Update, "Update"),
            (Action::Delete, "Delete"), (Action::Replace, "Replace"),
            (Action::Read, "Read"), (Action::Unchanged, "Unchanged"), (Action::Other, "Other"),
        ] {
            let members: Vec<_> = rows.iter().filter(|(node, _)| node.action == action).collect();
            if members.is_empty() { continue; }
            let start = y;
            let heading = format!("{label} ({})", members.len());
            let (_, stroke) = super::color(action);
            let mut content = format!(r##"<title>{heading}</title><text x="16" y="{}" font-size="13" font-weight="700" fill="{stroke}">{heading}</text>"##, start + 24);
            y += 32;
            for (node, lines) in members {
                let (row, height) = resource_row(graph, node, lines, width, y);
                content.push_str(&row);
                y += height;
            }
            writeln!(svg, r#"<g data-action-section="{}"><rect x="8" y="{start}" width="{}" height="{}" rx="8" fill="none" stroke="{stroke}" stroke-width="2"/>{content}</g>"#, super::label(action), width - 16, y - start).unwrap();
            y += 12;
        }
        svg.push_str("</g>\n");
        Card { width, height: y + 16, content: svg }
    }).collect()
}

#[cfg(test)]
#[path = "../../tests/unit/svg/logical.rs"]
mod tests;
