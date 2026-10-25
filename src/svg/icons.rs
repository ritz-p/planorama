use crate::{
    icons::{ResourceIcon, SIZE},
    model::Node,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) fn for_node(_node: &Node) -> Option<ResourceIcon> {
    None
}

pub(super) fn service_name(node: &Node, x: usize, y: usize, width: usize) -> String {
    let mut svg = String::new();
    if let Some(name) = node.service_name {
        let (category, background, foreground) = crate::services::category(node).palette();
        let lines = crate::view::service_label::lines(name, width);
        let badge_width = lines
            .iter()
            .map(|line| crate::view::text::width(line) * 8)
            .max()
            .unwrap_or(0)
            + crate::view::service_label::PADDING * 2;
        write!(svg, r##"<rect data-service-badge="true" data-service-category="{category}" data-service-full-name="{}" x="{x}" y="{}" width="{badge_width}" height="{}" rx="4" fill="{background}"/>"##,
            super::escape(name), y.saturating_sub(14), lines.len() * crate::view::service_label::LINE_HEIGHT - 2).unwrap();
        for (index, line) in lines.iter().enumerate() {
            write!(svg, r##"<text data-service-name="true" x="{}" y="{}" font-size="13" font-weight="700" fill="{foreground}" xml:space="preserve">{}</text>"##,
                x + crate::view::service_label::PADDING,
                y + index * crate::view::service_label::LINE_HEIGHT, super::escape(line)).unwrap();
        }
    }
    svg
}

#[cfg(test)]
#[path = "../../tests/unit/svg/icons.rs"]
mod tests;

pub(super) fn definitions(nodes: &[Node]) -> String {
    let used: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(for_node)
        .map(|icon| (icon.id(), icon))
        .collect();
    let mut svg = String::new();
    if !used.is_empty() {
        let notices = used
            .values()
            .map(|icon| icon.notice())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n");
        write!(
            svg,
            "<metadata id=\"planorama-icons-license\">{}</metadata>",
            super::escape(&notices.replace("\r\n", "\n"))
        )
        .unwrap();
    }
    for icon in used.values() {
        svg.push_str(icon.svg());
    }
    svg
}

pub(super) fn render(icon: ResourceIcon, x: usize, y: usize) -> String {
    format!(
        r##"<use href="#{}" x="{x}" y="{y}" width="{SIZE}" height="{SIZE}" aria-hidden="true"/>"##,
        icon.id()
    )
}
