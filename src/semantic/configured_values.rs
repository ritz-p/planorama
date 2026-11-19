use crate::model::{
    ArchitectureRelationship, Edge, EdgeKind, Graph, RelationshipProvenance, TerraformEntityId,
    TerraformPlan,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn apply(plan: &TerraformPlan, graph: &mut Graph) {
    if plan.configured_value_matches.is_empty() {
        return;
    }
    let indexes: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ((node.address.as_str(), node.deposed_key.as_deref()), index))
        .collect();
    let lookup = |id: &TerraformEntityId| {
        indexes
            .get(&(id.address.as_str(), id.deposed_key.as_deref()))
            .copied()
    };
    let mut facts: BTreeSet<_> = graph
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == EdgeKind::Connection && relationship.inferred)
        .flat_map(|relationship| {
            relationship.provenance.iter().filter_map(|proof| {
                let RelationshipProvenance::ConfiguredValue {
                    from,
                    to,
                    attribute,
                } = proof
                else {
                    return None;
                };
                let (from, to) = (lookup(from)?, lookup(to)?);
                (relationship.from == graph.nodes[from].entity.id
                    && relationship.to == graph.nodes[to].entity.id)
                    .then_some((from, to, *attribute))
            })
        })
        .collect();
    let mut edges: BTreeSet<_> = graph
        .edges
        .iter()
        .map(|edge| (edge.from, edge.to, edge.kind))
        .collect();
    for matched in &plan.configured_value_matches {
        let source = &plan.nodes[matched.source];
        let consumer = &plan.nodes[matched.consumer];
        let (Some(&from), Some(&to)) = (
            indexes.get(&(source.address.as_str(), source.deposed_key.as_deref())),
            indexes.get(&(consumer.address.as_str(), consumer.deposed_key.as_deref())),
        ) else {
            continue;
        };
        let kind = EdgeKind::Connection;
        if facts.insert((from, to, matched.attribute)) {
            graph.relationships.push(ArchitectureRelationship::new(
                graph.nodes[from].entity.id.clone(),
                graph.nodes[to].entity.id.clone(),
                kind,
                true,
                vec![RelationshipProvenance::ConfiguredValue {
                    from: source.into(),
                    to: consumer.into(),
                    attribute: matched.attribute,
                }],
            ));
        }
        if edges.insert((from, to, kind)) {
            graph.edges.push(Edge {
                from,
                to,
                kind,
                change: None,
            });
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/semantic/configured_values.rs"]
mod tests;
