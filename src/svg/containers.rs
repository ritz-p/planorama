use super::{color, escape};
use crate::{
    layout::Layout,
    model::{Action, Graph, ResourceRole},
};
use std::fmt::Write;

#[cfg(test)]
#[path = "../../tests/unit/svg/containers.rs"]
mod tests;

pub(super) fn background(graph: &Graph, layout: &Layout<'_>, node: usize) -> &'static str {
    let action = graph.nodes[node].action;
    if graph.nodes[node].role != ResourceRole::Container {
        return color(action).0;
    }
    let mut depth = 0;
    let mut parent = layout.containment.parents[node];
    while let Some(index) = parent {
        depth += 1;
        parent = layout.containment.parents[index];
    }
    let shades = match action {
        Action::Create => ["#ecfdf5", "#d1fae5", "#a7f3d0", "#8be8c3", "#6ee7b7"],
        Action::Update => ["#eff6ff", "#dbeafe", "#bfdbfe", "#abd0fd", "#93c5fd"],
        Action::Delete => ["#fef2f2", "#fee2e2", "#fecaca", "#fdb8b8", "#fca5a5"],
        Action::Replace => ["#fff7ed", "#ffedd5", "#fed7aa", "#fec58f", "#fdba74"],
        Action::Read => ["#faf5ff", "#f3e8ff", "#e9d5ff", "#e0c2fd", "#d8b4fe"],
        Action::Unchanged => ["#f8fafc", "#f1f5f9", "#e2e8f0", "#d5dee9", "#cbd5e1"],
        Action::Other => ["#fefce8", "#fef9c3", "#fef08a", "#fde86b", "#fde047"],
    };
    shades[depth.min(4)]
}

pub(super) fn boundaries(
    graph: &Graph,
    layout: &Layout<'_>,
    address_format: super::AddressFormat,
) -> String {
    let mut svg = String::new();
    let radius = crate::view::geometry::CONTAINER_RADIUS;
    let stroke = crate::view::geometry::CONTAINER_STROKE as f64 / 4.0;
    for &node in &layout.containers {
        let bounds = layout.bounds[node];
        let (x, y, width, height) = (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
        );
        let border = color(graph.nodes[node].action).1;
        let mode = super::entities::border(graph.nodes[node].mode);
        let background = background(graph, layout, node);
        let id = super::identity::resource(&graph.nodes[node]);
        let attribute = if graph.nodes[node].role == ResourceRole::Container {
            "data-container"
        } else if graph.nodes.iter().any(|child| {
            child.member_owner.is_some() && crate::view::routes::owner(graph, child) == Some(node)
        }) {
            "data-member-cluster"
        } else {
            "data-route-table"
        };
        writeln!(svg, r##"<g {attribute}="{id}"><title>{}</title><rect x="{x}" y="{y}" width="{width}" height="{height}" rx="{radius}" fill="{background}" stroke="{border}" stroke-width="{stroke}"{mode}/></g>"##, escape(&graph.nodes[node].resource_address().formatted(matches!(address_format, super::AddressFormat::Qualified)))).unwrap();
    }
    svg
}
