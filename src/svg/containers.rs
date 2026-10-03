use super::{color, escape};
use crate::{layout::Layout, model::Graph};
use std::fmt::Write;

pub(super) fn boundaries(graph: &Graph, layout: &Layout<'_>) -> String {
    let mut svg = String::new();
    for &(node, bounds) in &layout.containers {
        let (x, y, width, height) = (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
        );
        let border = color(graph.nodes[node].action).1;
        writeln!(svg, r##"<g data-container="resource-{node}"><title>{}</title><rect x="{x}" y="{y}" width="{width}" height="{height}" rx="12" fill="#ffffff" stroke="{border}" stroke-width="2"/></g>"##, escape(&graph.nodes[node].address)).unwrap();
    }
    svg
}
