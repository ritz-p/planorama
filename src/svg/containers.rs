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
    let mut parent = layout.parents[node];
    while let Some(index) = parent {
        depth += 1;
        parent = layout.parents[index];
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

pub(super) fn boundaries(graph: &Graph, layout: &Layout<'_>) -> String {
    let mut svg = String::new();
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
        writeln!(svg, r##"<g data-container="{id}"><title>{}</title><rect x="{x}" y="{y}" width="{width}" height="{height}" rx="12" fill="{background}" stroke="{border}" stroke-width="2"{mode}/></g>"##, escape(&graph.nodes[node].address)).unwrap();
    }
    svg
}
