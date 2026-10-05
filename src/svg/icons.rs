use crate::{
    icons::{Provider, ResourceIcon, icon_for},
    model::Node,
};
use std::collections::BTreeMap;
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/icons.rs"]
mod tests;

pub(super) const SIZE: usize = 24;
pub(super) const GAP: usize = 8;
const NOTICE: &str = include_str!("../../assets/icons/aws/NOTICE.txt");

pub(super) fn for_node(node: &Node) -> Option<ResourceIcon> {
    icon_for(
        Provider::from_resource_type(&node.resource_type),
        &node.resource_type,
    )
}

pub(super) fn definitions(nodes: &[Node]) -> String {
    let used: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(for_node)
        .map(|icon| (icon.id(), icon))
        .collect();
    let mut svg = String::new();
    if !used.is_empty() {
        write!(
            svg,
            "<metadata id=\"planorama-icons-license\">{}</metadata>",
            super::escape(&NOTICE.replace("\r\n", "\n"))
        )
        .unwrap();
    }
    for (id, icon) in used {
        let data = data_uri(icon.svg());
        write!(svg, r#"<symbol id="{id}" viewBox="0 0 24 24"><image width="24" height="24" preserveAspectRatio="xMidYMid meet" href="{data}"/></symbol>"#).unwrap();
    }
    svg
}

fn data_uri(svg: &str) -> String {
    let mut result = String::from("data:image/svg+xml,");
    for byte in svg.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char)
            }
            _ => write!(result, "%{byte:02X}").unwrap(),
        }
    }
    result
}

pub(super) fn render(icon: ResourceIcon, x: usize, y: usize) -> String {
    format!(
        r##"<use href="#{}" x="{x}" y="{y}" width="{SIZE}" height="{SIZE}" aria-hidden="true"/>"##,
        icon.id()
    )
}
