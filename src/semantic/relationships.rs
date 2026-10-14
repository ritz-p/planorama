use crate::model::{
    ArchitectureRelationship, EdgeKind, Graph, RelationshipProvenance, TerraformEntityId,
};
use std::collections::BTreeMap;

pub(crate) fn reference(graph: &Graph, from: usize, to: usize) -> RelationshipProvenance {
    let identity = |index: usize| TerraformEntityId {
        address: graph.nodes[index].address.clone(),
        deposed_key: graph.nodes[index].deposed_key.clone(),
    };
    RelationshipProvenance::Reference {
        from: identity(from),
        to: identity(to),
    }
}

pub(crate) fn record(
    graph: &mut Graph,
    from: usize,
    to: usize,
    kind: EdgeKind,
    provenance: Vec<RelationshipProvenance>,
) {
    graph.relationships.push(ArchitectureRelationship::new(
        graph.nodes[from].entity.id.clone(),
        graph.nodes[to].entity.id.clone(),
        kind,
        true,
        provenance,
    ));
}

pub(crate) fn record_reference(graph: &mut Graph, from: usize, to: usize, kind: EdgeKind) {
    let evidence = vec![reference(graph, from, to)];
    record(graph, from, to, kind, evidence);
}

pub(super) fn deployments(
    nodes: &[crate::model::Node],
) -> BTreeMap<TerraformEntityId, crate::model::ResourceDeployment> {
    nodes
        .iter()
        .map(|node| {
            (
                TerraformEntityId {
                    address: node.address.clone(),
                    deposed_key: node.deposed_key.clone(),
                },
                crate::model::ResourceDeployment {
                    scope: node.entity.scope.clone(),
                    provider: node.provider.clone(),
                    configuration: node.provider_configuration.clone(),
                },
            )
        })
        .collect()
}

pub(super) fn collect(
    graph: &Graph,
    deployments: &BTreeMap<TerraformEntityId, crate::model::ResourceDeployment>,
) -> Vec<ArchitectureRelationship> {
    let mut evidence = BTreeMap::<_, Vec<_>>::new();
    for relationship in graph.relationships.iter().filter(|r| r.inferred) {
        evidence
            .entry((&relationship.from, &relationship.to, relationship.kind))
            .or_default()
            .extend(relationship.provenance.iter());
    }
    let mut relationships: Vec<_> = graph
        .edges
        .iter()
        .map(|edge| {
            let from = &graph.nodes[edge.from].entity.id;
            let to = &graph.nodes[edge.to].entity.id;
            let provenance = if let Some(change) = &edge.change {
                let source = TerraformEntityId {
                    address: change.address.clone(),
                    deposed_key: (graph.nodes[edge.from].address == change.address)
                        .then(|| graph.nodes[edge.from].deposed_key.clone())
                        .flatten(),
                };
                vec![RelationshipProvenance::Resource {
                    deployment: deployments.get(&source).cloned().map(Box::new),
                    source,
                    change: change.clone(),
                }]
            } else if edge.kind == EdgeKind::Dependency {
                vec![reference(graph, edge.from, edge.to)]
            } else {
                evidence
                    .get(&(from, to, edge.kind))
                    .map_or_else(Vec::new, |sources| {
                        sources.iter().map(|source| (*source).clone()).collect()
                    })
            };
            ArchitectureRelationship::new(
                from.clone(),
                to.clone(),
                edge.kind,
                edge.kind != EdgeKind::Dependency && edge.change.is_none(),
                provenance,
            )
        })
        .collect();
    relationships.sort();
    relationships.dedup();
    relationships
}
