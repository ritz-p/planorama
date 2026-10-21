use crate::model::architecture::{Graph, Node};
use crate::view::references::{LINE_HEIGHT, Row, rows, text_width};
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/references.rs"]
mod tests;

fn description(row: &Row<'_>) -> String {
    if row.reference.kind == crate::model::ReferenceKind::Information {
        return row.reference.label.into();
    }
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
    let mut row_y = y + LINE_HEIGHT;
    let text_width = text_width(node).min(width);
    for row in &rows {
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
        let mut href = remote
            .map(|(target, state)| {
                format!(
                    "{}{}",
                    super::states::prefix(state),
                    super::identity::source(&target.id)
                )
            })
            .or_else(|| available.map(super::identity::resource));
        if let crate::model::ReferenceKind::OrganizationRoot(index) = row.reference.kind {
            href = row.target.and_then(|target| {
                let id = crate::provider::aws::organizations::root_id(&target.id.address, index);
                graph
                    .components
                    .iter()
                    .find(|component| component.entity.id == id)
                    .map(|component| super::identity::architecture(&component.entity.id))
            });
        }
        let label = row.label(href.is_some());
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
        } else if row.reference.kind == crate::model::ReferenceKind::Information {
            svg.push_str("<g>");
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
        write!(svg, "<title>{}</title>", super::escape(&description(row))).unwrap();
        for (line, text) in row.lines(&label, text_width).iter().enumerate() {
            writeln!(
                svg,
                r##"<text x="{x}" y="{}" font-size="11" fill="#334155" xml:space="preserve">{}</text>"##,
                row_y + line * LINE_HEIGHT,
                super::escape(text)
            )
            .unwrap();
        }
        svg.push_str(if href.is_some() { "</a>" } else { "</g>" });
        row_y += row.line_count(text_width) * LINE_HEIGHT;
    }
    svg.push_str("</g>\n");
    svg
}
