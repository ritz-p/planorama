use super::annotations::{Card, card_width, wrap};
use crate::model::architecture::Graph;
use std::collections::BTreeMap;
use std::fmt::Write;

pub(super) fn cards(graph: &Graph, format: super::AddressFormat) -> Vec<Card> {
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for node in &graph.nodes {
        if !node.provider.is_cloudflare() {
            continue;
        }
        let scope = node.entity.cloudflare_scope.as_ref();
        groups
            .entry((
                node.provider_configuration.as_ref(),
                scope.and_then(|s| s.account.as_deref()),
                scope.and_then(|s| s.zone.as_deref()),
            ))
            .or_default()
            .push(node);
    }
    groups
        .into_iter()
        .map(|((provider, account, zone), mut nodes)| {
            nodes.sort_by_key(|node| &node.entity.id);
            let heading = format!(
                "Account {} / Zone {}",
                account.unwrap_or("unknown"),
                zone.unwrap_or("unknown")
            );
            let provider = format!(
                "Provider configuration: {}",
                provider.map_or("unknown", |p| p.key.as_str())
            );
            let mut rows = vec![
                heading,
                provider,
                "Administrative index; not physical containment.".into(),
            ];
            let members: Vec<_> = nodes
                .iter()
                .map(|node| {
                    format!(
                        "{}; {}{}",
                        node.resource_address()
                            .formatted(matches!(format, super::AddressFormat::Qualified)),
                        super::operations::label(node.action, &node.metadata),
                        node.deposed_key
                            .as_ref()
                            .map(|key| format!("; deposed: {key}"))
                            .unwrap_or_default()
                    )
                })
                .collect();
            let width = card_width(rows.iter().chain(&members).map(String::as_str));
            let limit = (width - 32) / 8;
            let mut content = String::from("<g data-cloudflare-management-group=\"true\">\n");
            let mut y = 28;
            for (index, row) in rows.drain(..).enumerate() {
                for line in wrap(&row, limit) {
                    writeln!(
                        content,
                        "<text x=\"16\" y=\"{y}\" font-size=\"12\" font-weight=\"{}\">{}</text>",
                        if index == 0 { 700 } else { 400 },
                        super::escape(line)
                    )
                    .unwrap();
                    y += 20;
                }
            }
            for (node, text) in nodes.iter().zip(members) {
                write!(
                    content,
                    "<a data-cloudflare-management-member=\"true\" href=\"#{}\"><title>{}</title>",
                    super::identity::resource(node),
                    super::escape(&text)
                )
                .unwrap();
                for line in crate::view::text::wrap_address(&text, limit) {
                    writeln!(
                        content,
                        r##"<text x="16" y="{y}" font-size="12" fill="#334155">{}</text>"##,
                        super::escape(line)
                    )
                    .unwrap();
                    y += 20;
                }
                content.push_str("</a>\n");
                y += 12;
            }
            content.push_str("</g>\n");
            Card {
                width,
                height: y + 16,
                content,
            }
        })
        .collect()
}
