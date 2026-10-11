use crate::layout::{Layout, Point};
use crate::model::{Directionality, EdgeKind, Graph};
use std::fmt::Write;

pub(super) fn circle(point: Point, end: &str, stroke: &str) -> String {
    format!(
        r##"<circle data-endpoint="{end}" cx="{}" cy="{}" r="3.5" fill="#ffffff" stroke="{stroke}" stroke-width="1.5"/>"##,
        point.x, point.y
    )
}

pub(super) fn render(graph: &Graph, layout: &Layout<'_>) -> String {
    let mut svg = String::new();
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        if edge.kind == EdgeKind::Containment || path.len() < 2 {
            continue;
        }
        let stroke = super::relationships::stroke(edge);
        let source = super::identity::resource(&graph.nodes[edge.from]);
        let target = super::identity::resource(&graph.nodes[edge.to]);
        writeln!(svg, r#"<g data-endpoints="true" data-source="{source}" data-target="{target}" aria-hidden="true">"#).unwrap();
        svg.push_str(&circle(path[0], "source", stroke));
        if matches!(edge.kind, EdgeKind::Association | EdgeKind::Connection) {
            let end = path[path.len() - 1];
            svg.push_str(&circle(end, "target", stroke));
            if edge.directionality() == Directionality::Directed {
                if let Some(previous) = path.iter().rev().find(|&&point| point != end) {
                    let dx = end.x.cmp(&previous.x) as i8 as f64;
                    let dy = end.y.cmp(&previous.y) as i8 as f64;
                    let (x, y) = (end.x as f64, end.y as f64);
                    writeln!(svg, r#"<path data-endpoint-arrow="true" d="M {} {} L {} {} L {} {} Z" fill="{stroke}"/>"#,
                        x - 6.3 * dx + 3.5 * dy, y - 6.3 * dy - 3.5 * dx,
                        x + 0.7 * dx, y + 0.7 * dy,
                        x - 6.3 * dx - 3.5 * dy, y - 6.3 * dy + 3.5 * dx).unwrap();
                }
            }
        }
        svg.push_str("</g>\n");
    }
    svg
}

#[cfg(test)]
#[path = "../../tests/unit/svg/endpoints.rs"]
mod tests;
