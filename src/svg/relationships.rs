use super::{color, escape, label};
use crate::model::architecture::{Edge, EdgeKind, Graph};
use std::collections::BTreeSet;
use std::fmt::Write;

pub(super) fn stroke(edge: &Edge) -> &'static str {
    edge.change
        .as_ref()
        .map_or("#94a3b8", |change| color(change.action).1)
}

pub(super) fn style(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Dependency | EdgeKind::Containment => r#"stroke-width="1.5""#,
        EdgeKind::Association => r#"stroke-width="2" stroke-dasharray="7 4""#,
        EdgeKind::Connection => {
            r#"stroke-width="2.5" stroke-dasharray="0 3.5" stroke-linecap="round""#
        }
    }
}

pub(super) fn captions(graph: &Graph) -> (String, String) {
    let nodes = graph.nodes.len();
    let edges = graph.edges.len();
    let logical = graph
        .nodes
        .iter()
        .filter(|n| n.placement == crate::model::Placement::Logical)
        .count();
    let embedded = graph
        .nodes
        .iter()
        .filter(|n| crate::view::routes::owner(graph, n).is_some())
        .count();
    if embedded > 0 {
        let resources = nodes + additional_resources(graph);
        let spatial = nodes - logical - embedded;
        let summary = format!(
            "{resources} resources ({spatial} spatial cards, {logical} logical cards, {embedded} embedded route cards) · {edges} relationships"
        );
        return (summary.clone(), summary);
    }
    if logical > 0 {
        let resources = nodes + additional_resources(graph);
        let spatial = nodes - logical;
        let summary = format!(
            "{resources} resources ({spatial} spatial cards, {logical} logical cards) · {edges} relationships"
        );
        return (summary.clone(), summary);
    }
    match graph
        .edges
        .iter()
        .any(|edge| edge.kind != EdgeKind::Dependency)
    {
        false => (
            format!(
                "{nodes} resources, {edges} reference edges. Arrows point from dependencies to dependent resources. References are conservative, not Terraform execution order."
            ),
            format!("{nodes} resources · {edges} reference edges · dependency → dependent"),
        ),
        true => {
            let resources = nodes + additional_resources(graph);
            (
                format!(
                    "{resources} resources ({nodes} cards), {edges} relationships. Lines show architectural relationships. Edge titles identify the relationship kind and Terraform change."
                ),
                format!("{resources} resources ({nodes} cards) · {edges} relationships"),
            )
        }
    }
}

fn additional_resources(graph: &Graph) -> usize {
    graph
        .edges
        .iter()
        .filter(|e| {
            e.change
                .as_ref()
                .is_some_and(|c| graph.nodes[e.from].address != c.address)
        })
        .count()
}

pub(super) fn title(graph: &Graph, edge: &Edge, semantic_edges: bool) -> String {
    let mut title = format!(
        "{} {} {}",
        graph.nodes[edge.from].address,
        if edge.directionality() == crate::model::Directionality::Undirected {
            "↔"
        } else {
            "→"
        },
        graph.nodes[edge.to].address
    );
    let kind = match edge.kind {
        EdgeKind::Dependency => "dependency",
        EdgeKind::Association => "association",
        EdgeKind::Connection => "connection",
        EdgeKind::Containment => "containment",
    };
    match &edge.change {
        Some(change) => write!(
            title,
            " ({kind}; {}: {})",
            super::operations::label(change.action, &change.metadata),
            change.address
        )
        .unwrap(),
        None if semantic_edges => write!(title, " ({kind})").unwrap(),
        None => {}
    }
    if let Some(change) = &edge.change {
        title.push_str(&super::operations::details(&change.metadata));
    }
    escape(&title)
}

pub(super) fn markers(graph: &Graph) -> String {
    let mut markers = String::new();
    let actions: BTreeSet<_> = graph
        .edges
        .iter()
        .filter_map(|edge| edge.change.as_ref().map(|change| change.action))
        .collect();
    for action in actions {
        let name = label(action);
        let border = color(action).1;
        write!(markers, r##"<marker id="arrow-{name}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" markerUnits="userSpaceOnUse" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="{border}"/></marker>"##).unwrap();
    }
    markers
}
