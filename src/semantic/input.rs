use crate::model::{AttributeReference, DriftRecord, Graph, Node, StateOutput, TerraformPlan};

pub(crate) struct Input {
    pub graph: Graph,
    pub attributes: Vec<AttributeReference>,
    pub graph_references: Vec<AttributeReference>,
    pub outputs: Vec<StateOutput>,
    pub drift: Vec<DriftRecord>,
}

impl std::ops::Deref for Input {
    type Target = Graph;
    fn deref(&self) -> &Graph {
        &self.graph
    }
}

impl Input {
    pub fn new(plan: &TerraformPlan) -> Self {
        Self {
            graph: base_graph(plan),
            attributes: plan.attributes.clone(),
            graph_references: plan.graph_references.clone(),
            outputs: plan.outputs.clone(),
            drift: plan.drift.clone(),
        }
    }
}

pub(crate) fn base_graph(plan: &TerraformPlan) -> Graph {
    let mut graph = Graph {
        relationships: Vec::new(),
        components: Vec::new(),
        checks: plan.checks.clone(),
        status: plan.status,
        nodes: plan
            .nodes
            .iter()
            .enumerate()
            .map(|(index, fact)| Node {
                placement: crate::provider::placement(&fact.provider, &fact.resource_type),
                references: plan
                    .attributes
                    .iter()
                    .map(|r| (r, false))
                    .chain(plan.graph_references.iter().map(|r| (r, true)))
                    .filter(|(r, _)| {
                        r.target == index && (!r.sources.is_empty() || !r.issues.is_empty())
                    })
                    .map(|(r, dependency)| crate::model::ResourceReference {
                        label: crate::provider::reference_label(
                            &fact.provider,
                            &fact.resource_type,
                            &r.attribute,
                            dependency,
                        ),
                        attribute: r.attribute.clone(),
                        targets: r
                            .sources
                            .iter()
                            .map(|&i| crate::model::ReferenceTarget {
                                state: None,
                                id: (&plan.nodes[i]).into(),
                                placement: crate::provider::placement(
                                    &plan.nodes[i].provider,
                                    &plan.nodes[i].resource_type,
                                ),
                            })
                            .collect(),
                        complete: r.complete,
                        dependency,
                        issues: r.issues.clone(),
                    })
                    .collect(),
                entity: {
                    let mut entity = crate::model::ArchitectureEntity::terraform(fact.into());
                    entity.scope = crate::provider::scope(fact);
                    entity
                },
                address: fact.address.clone(),
                deposed_key: fact.deposed_key.clone(),
                previous_address: fact.previous_address.clone(),
                metadata: fact.metadata.clone(),
                resource_type: fact.resource_type.clone(),
                provider: fact.provider.clone(),
                provider_configuration: fact.provider_configuration.clone(),
                module: fact.module.clone(),
                action: fact.action,
                mode: fact.mode,
                role: crate::classification::classify(&fact.provider, &fact.resource_type),
            })
            .collect(),
        edges: plan
            .edges
            .iter()
            .map(|edge| edge.endpoints().into())
            .collect(),
    };
    graph.relationships = super::relationships::collect(&graph);
    graph
}
