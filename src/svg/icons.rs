use crate::{
    icons::{Provider, ResourceIcon, icon_for},
    model::Node,
};
use std::collections::BTreeSet;
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/icons.rs"]
mod tests;

pub(super) const SIZE: usize = 24;
pub(super) const GAP: usize = 8;
const LICENSE: &str = include_str!("../../docs/icons-LICENSE.txt");

pub(super) fn for_node(node: &Node) -> Option<ResourceIcon> {
    icon_for(
        Provider::from_resource_type(&node.resource_type),
        &node.resource_type,
    )
}

pub(super) fn definitions(nodes: &[Node]) -> String {
    let used: BTreeSet<_> = nodes.iter().filter_map(for_node).collect();
    let mut svg = String::new();
    if !used.is_empty() {
        write!(
            svg,
            "<metadata id=\"planorama-icons-license\">{}</metadata>",
            super::escape(LICENSE)
        )
        .unwrap();
    }
    for icon in used {
        write!(svg, r##"<symbol id="{}" viewBox="0 0 24 24"><rect width="24" height="24" rx="4" fill="{}"/><g fill="none" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{}</g></symbol>"##, icon.id(), icon.color(), icon.shapes()).unwrap();
    }
    svg
}

pub(super) fn render(icon: ResourceIcon, x: usize, y: usize) -> String {
    format!(
        r##"<use href="#{}" x="{x}" y="{y}" width="{SIZE}" height="{SIZE}" aria-hidden="true"/>"##,
        icon.id()
    )
}
