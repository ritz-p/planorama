use crate::{
    icons::{ResourceIcon, SIZE},
    model::Node,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) fn for_node(node: &Node) -> Option<ResourceIcon> {
    if node.service_name.is_some() {
        None
    } else {
        crate::icons::for_node(node)
    }
}

pub(super) fn service_name(node: &Node, x: usize, y: usize, width: usize) -> String {
    let mut svg = String::new();
    if let Some(name) = node.service_name {
        for (index, line) in crate::view::text::wrap(name, (width / 8).max(1))
            .iter()
            .enumerate()
        {
            write!(svg, r##"<text data-service-name="true" x="{x}" y="{}" font-size="13" fill="#0f172a" xml:space="preserve">{}</text>"##, y + index * 20, super::escape(line)).unwrap();
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
