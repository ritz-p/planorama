use crate::{
    icons::{ResourceIcon, SIZE},
    model::Node,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) fn for_node(_node: &Node) -> Option<ResourceIcon> {
    None
}

pub(super) fn service_name(node: &Node, right: usize, y: usize, width: usize) -> String {
    let mut svg = String::new();
    if let Some(name) = crate::services::display_name(node) {
        use crate::view::service_label::{FONT_SIZE, LINE_HEIGHT, PADDING, VERTICAL_PADDING};
        let (category, background, foreground) = crate::services::category(node).palette();
        let lines = crate::view::service_label::lines(name, width);
        let badge_width = crate::view::service_label::width(name, width);
        let x = right.saturating_sub(badge_width);
        write!(svg, r##"<rect data-service-badge="true" data-service-category="{category}" data-service-full-name="{}" data-display-name="{}" x="{x}" y="{y}" width="{badge_width}" height="{}" rx="4" fill="{background}"/>"##,
            super::escape(node.service_name.unwrap()), super::escape(name), lines.len() * LINE_HEIGHT + VERTICAL_PADDING * 2).unwrap();
        for (index, line) in lines.iter().enumerate() {
            write!(svg, r##"<text data-service-name="true" x="{}" y="{}" font-size="{FONT_SIZE}" font-weight="700" fill="{foreground}" xml:space="preserve">{}</text>"##,
                x + PADDING,
                y + VERTICAL_PADDING + FONT_SIZE + index * LINE_HEIGHT, super::escape(line)).unwrap();
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
