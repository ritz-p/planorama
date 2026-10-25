use super::{color, escape, label};
use crate::model::Directionality;
use crate::model::architecture::{Action, Edge, EdgeKind, Graph};
use std::collections::BTreeSet;
use std::fmt::Write;

pub(super) fn stroke(edge: &Edge) -> &'static str {
    action_stroke(edge.change.as_ref().map(|change| change.action))
}

pub(super) fn action_stroke(action: Option<Action>) -> &'static str {
    action.map_or("#94a3b8", |action| color(action).1)
}

fn marker_id(action: Option<Action>) -> String {
    action.map_or_else(
        || "arrow".into(),
        |action| format!("arrow-{}", label(action)),
    )
}

pub(super) fn arrow(directionality: Directionality, action: Option<Action>) -> String {
    if directionality == Directionality::Directed {
        format!(r#" marker-end="url(#{})""#, marker_id(action))
    } else {
        String::new()
    }
}

pub(super) fn style(kind: EdgeKind) -> &'static str {
    crate::view::geometry::relationship_style(kind)
}

pub(super) fn captions(graph: &Graph) -> (String, String) {
    let nodes = graph.nodes.len();
    let edges = graph.edges.len();
    let logical = graph
        .nodes
        .iter()
        .filter(|n| n.placement == crate::model::Placement::Logical)
        .count();
    let logical_groups = super::logical::group_count(graph);
    let embedded = graph
        .nodes
        .iter()
        .filter(|n| crate::view::routes::owner(graph, n).is_some())
        .count();
    if embedded > 0 {
        let resources = nodes + additional_resources(graph);
        let spatial = nodes - logical - embedded;
        let members = graph
            .nodes
            .iter()
            .filter(|node| {
                node.member_owner.is_some() && crate::view::routes::owner(graph, node).is_some()
            })
            .count();
        let embedded_label = if members == 0 {
            format!("{embedded} embedded route cards")
        } else {
            format!(
                "{} embedded route cards, {members} member cards",
                embedded - members
            )
        };
        let summary = format!(
            "{resources} resources ({spatial} spatial cards, {logical} logical resources in {logical_groups} group cards, {embedded_label}) · {edges} relationships"
        );
        return (summary.clone(), summary);
    }
    if logical > 0 {
        let resources = nodes + additional_resources(graph);
        let spatial = nodes - logical;
        let summary = format!(
            "{resources} resources ({spatial} spatial cards, {logical} logical resources in {logical_groups} group cards) · {edges} relationships"
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
            e.change.as_ref().is_some_and(|c| {
                !graph
                    .nodes
                    .iter()
                    .any(|n| n.address == c.address && n.deposed_key.is_none())
            })
        })
        .count()
}

pub(super) fn title(
    graph: &Graph,
    edge: &Edge,
    semantic_edges: bool,
    address_format: super::AddressFormat,
) -> String {
    let qualified = matches!(address_format, super::AddressFormat::Qualified);
    let mut title = format!(
        "{} {} {}",
        graph.nodes[edge.from]
            .resource_address()
            .formatted(qualified),
        if edge.directionality() == crate::model::Directionality::Undirected {
            "↔"
        } else {
            "→"
        },
        graph.nodes[edge.to].resource_address().formatted(qualified)
    );
    let kind = match edge.kind {
        EdgeKind::Dependency => "dependency",
        EdgeKind::Association => "association",
        EdgeKind::Subscription => "subscription",
        EdgeKind::EventSource => "event source",
        EdgeKind::EventTarget => "event target",
        EdgeKind::Connection => "connection",
        EdgeKind::Containment => "containment",
    };
    match &edge.change {
        Some(change) => write!(
            title,
            " ({kind}; {}: {})",
            super::operations::label(change.action, &change.metadata),
            change.resource_address().formatted(qualified)
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
    let actions: BTreeSet<_> = std::iter::once(None)
        .chain(
            graph
                .edges
                .iter()
                .map(|edge| edge.change.as_ref().map(|change| change.action)),
        )
        .collect();
    for action in actions {
        let name = marker_id(action);
        let border = action_stroke(action);
        write!(markers, r##"<marker id="{name}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" markerUnits="userSpaceOnUse" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="{border}"/></marker>"##).unwrap();
    }
    markers
}
