use crate::model::architecture::{Graph, Node};
use crate::view::references::{Row, VISIBLE_LIMIT, rows};
use std::fmt::Write;

fn description(row: &Row<'_>) -> String {
    let reference = row.reference;
    let target = row.target.map_or_else(
        || "unresolved".into(),
        |t| match &t.id.deposed_key {
            Some(key) => format!("{} (deposed: {key})", t.id.address),
            None => match &t.state {
                Some(state) => format!("{}:{}", state.as_str(), t.id.address),
                None => t.id.address.clone(),
            },
        },
    );
    format!(
        "{}: {target} [{}; {}]",
        reference.label,
        reference.attribute,
        if reference.dependency {
            "graph dependency"
        } else if reference.label == "Monitored by" {
            "verified inverse monitoring reference"
        } else if reference.label == "Monitors" && row.target.is_some() {
            "verified monitoring reference"
        } else if row.target.is_none() {
            "unresolved attribute reference"
        } else if row.target.is_some_and(|t| t.state.is_some()) {
            "resolved remote output reference"
        } else {
            "direct attribute reference"
        }
    )
}

pub(super) fn render(graph: &Graph, node: &Node, x: usize, y: usize, width: usize) -> String {
    let rows = rows(node);
    if rows.is_empty() {
        return String::new();
    }
    let complete = rows.iter().map(description).collect::<Vec<_>>().join("\n");
    let heading = if rows.iter().all(|r| r.reference.dependency) {
        "Dependencies"
    } else if rows.iter().any(|r| r.reference.dependency) {
        "Uses / dependencies"
    } else {
        "Uses"
    };
    let mut svg = format!(
        r##"<g data-resource-references="true"><title>{}</title><text x="{x}" y="{y}" font-size="11" font-weight="700" fill="#475569">{heading}</text>"##,
        super::escape(&complete)
    );
    for (index, row) in rows.iter().take(VISIBLE_LIMIT).enumerate() {
        let available = row.target.and_then(|target| {
            if target.state.is_some() {
                return None;
            }
            graph
                .nodes
                .iter()
                .find(|n| n.entity.provenance.contains(&target.id))
        });
        let remote = row
            .target
            .and_then(|target| target.state.as_ref().map(|state| (target, state)));
        let href = remote
            .map(|(target, state)| {
                format!(
                    "{}{}",
                    super::states::prefix(state),
                    super::identity::source(&target.id)
                )
            })
            .or_else(|| available.map(super::identity::resource));
        let label = match (row.target, available) {
            (Some(target), _) if remote.is_some() => format!(
                "{}: {}:{}",
                row.reference.label,
                target.state.as_ref().unwrap().as_str(),
                target.id.address
            ),
            (_, Some(target)) => format!(
                "{}: {}",
                row.reference.label,
                target.resource_address().local()
            ),
            (Some(target), None) => format!(
                "Unavailable — {}: {}",
                row.reference.label, target.id.address
            ),
            (None, _) => format!("{}: unresolved", row.reference.label),
        };
        if let Some(href) = &href {
            write!(
                svg,
                "<a {}=\"#{}\" data-reference-attribute=\"{}\">",
                if remote.is_some() {
                    "data-global-href"
                } else {
                    "href"
                },
                href,
                super::escape(&row.reference.attribute)
            )
            .unwrap();
        } else {
            write!(
                svg,
                "<g data-reference-status=\"{}\">",
                if row.target.is_some() {
                    "unavailable"
                } else {
                    "unresolved"
                }
            )
            .unwrap();
        }
        writeln!(
            svg,
            r##"<title>{}</title><text x="{x}" y="{}" font-size="11" fill="#334155">{}</text>{}"##,
            super::escape(&description(row)),
            y + 18 * (index + 1),
            super::escape(&super::shorten(&label, (width / 8).max(1))),
            if href.is_some() { "</a>" } else { "</g>" }
        )
        .unwrap();
    }
    if rows.len() > VISIBLE_LIMIT {
        writeln!(svg, r##"<text x="{x}" y="{}" font-size="11" fill="#64748b">+{} more (see full title)</text>"##, y + 18 * (VISIBLE_LIMIT + 1), rows.len() - VISIBLE_LIMIT).unwrap();
    }
    svg.push_str("</g>\n");
    svg
}
