use crate::model::{Action, EdgeKind, Graph};
use std::collections::BTreeSet;
use std::fmt::Write;

pub(super) fn height(graph: &Graph) -> usize {
    if graph
        .edges
        .iter()
        .any(|edge| edge.kind != EdgeKind::Containment)
    {
        72
    } else {
        0
    }
}

pub(super) fn render(graph: &Graph) -> String {
    let mut svg = String::from(
        "<g id=\"relationship-legend\" font-size=\"12\" fill=\"#475569\">\n<text x=\"40\" y=\"157\">Relationship lines: color = relationship action (not endpoint actions)</text>\n",
    );
    let actions: BTreeSet<Option<Action>> = std::iter::once(None)
        .chain(
            graph
                .edges
                .iter()
                .filter(|edge| edge.kind != EdgeKind::Containment)
                .map(|edge| edge.change.as_ref().map(|change| change.action)),
        )
        .collect();
    let mut x = 40;
    for action in actions {
        let (color, label, width) = match action {
            None => ("#94a3b8", "no independent change", 260),
            Some(action) => (super::color(action).1, super::label(action), 108),
        };
        writeln!(svg, "<path d=\"M {x} 177 H {}\" stroke=\"{color}\" stroke-width=\"2\"/><text x=\"{}\" y=\"181\">{label}</text>", x+26, x+33).unwrap();
        x += width;
    }
    svg.push_str("<text x=\"40\" y=\"205\">Line style:</text>");
    for (i, (kind, label)) in [
        (EdgeKind::Dependency, "dependency"),
        (EdgeKind::Association, "association"),
        (EdgeKind::Connection, "connection"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 140 + i * 200;
        writeln!(svg, "<path d=\"M {x} 201 H {}\" stroke=\"#64748b\" {}/><text x=\"{}\" y=\"205\">{label}</text>",x+30,super::relationships::style(kind),x+38).unwrap();
    }
    svg.push_str("</g>\n");
    svg
}
