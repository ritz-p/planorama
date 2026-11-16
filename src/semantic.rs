use crate::model::{ArchitectureGraph, TerraformPlan};
pub(crate) mod input;
pub(crate) use input::Input;
#[cfg(test)]
pub(crate) use tests::base_graph;
pub mod cross_state;
pub mod diagnostics;
pub(crate) mod relationships;

#[cfg(test)]
#[path = "../tests/unit/semantic.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/semantic/resolution.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "../tests/unit/semantic/eventbridge.rs"]
mod eventbridge_tests;

pub(crate) fn transform(plan: &TerraformPlan) -> ArchitectureGraph {
    let _timing = crate::timing::span(
        "semantic_transform",
        &[
            ("resources", plan.nodes.len()),
            ("references", plan.edges.len()),
        ],
    );
    let raw = Input::new(plan);
    let deployments = relationships::deployments(&raw.nodes);
    let mut graph = crate::provider::transform(raw);
    graph.relationships = relationships::collect(&graph, &deployments);
    if !plan.configured_value_matches.is_empty() {
        let mut existing: std::collections::BTreeSet<_> =
            graph.edges.iter().map(|e| (e.from, e.to, e.kind)).collect();
        let indexes: std::collections::BTreeMap<_, _> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| ((n.address.as_str(), n.deposed_key.as_deref()), i))
            .collect();
        for matched in &plan.configured_value_matches {
            let source = &plan.nodes[matched.source];
            let consumer = &plan.nodes[matched.consumer];
            let index = |id: &crate::model::TerraformEntity| {
                indexes
                    .get(&(id.address.as_str(), id.deposed_key.as_deref()))
                    .copied()
            };
            let (Some(from), Some(to)) = (index(source), index(consumer)) else {
                continue;
            };
            let provenance = crate::model::RelationshipProvenance::ConfiguredValue {
                from: source.into(),
                to: consumer.into(),
                attribute: matched.attribute,
            };
            graph
                .relationships
                .push(crate::model::ArchitectureRelationship::new(
                    graph.nodes[from].entity.id.clone(),
                    graph.nodes[to].entity.id.clone(),
                    crate::model::EdgeKind::Connection,
                    true,
                    vec![provenance],
                ));
            let edge = crate::model::Edge {
                from,
                to,
                kind: crate::model::EdgeKind::Connection,
                change: None,
            };
            if existing.insert((from, to, edge.kind)) {
                graph.edges.push(edge);
            }
        }
    }
    graph.coverage = crate::coverage::Coverage::collect(plan, &graph);
    ArchitectureGraph(graph)
}
