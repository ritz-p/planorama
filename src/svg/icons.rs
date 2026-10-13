pub(super) use crate::icons::for_node;
use crate::{
    icons::{ResourceIcon, SIZE},
    model::Node,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

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
