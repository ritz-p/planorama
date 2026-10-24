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
        coverage: Default::default(),
        relationships: Vec::new(),
        components: crate::provider::aws::organizations::roots(plan),
        checks: plan.checks.clone(),
        status: plan.status,
        nodes: plan
            .nodes
            .iter()
            .enumerate()
            .map(|(index, fact)| Node {
                member_owner: None,
                service_name: None,
                routes: crate::model::RoutePresentation {
                    entries: fact.route_entries.clone(),
                    ..Default::default()
                },
                placement: if crate::provider::aws::organizations::unsupported_data(
                    &fact.provider,
                    &fact.resource_type,
                    fact.mode,
                ) {
                    crate::model::Placement::Unresolved
                } else {
                    crate::provider::entity_placement(fact)
                },
                references: plan
                    .attributes
                    .iter()
                    .map(|r| (r, false))
                    .chain(plan.graph_references.iter().map(|r| (r, true)))
                    .filter(|(r, _)| {
                        r.target == index && (r.has_references || !r.issues.is_empty())
                    })
                    .map(|(r, dependency)| crate::model::ResourceReference {
                        kind: crate::model::ReferenceKind::Configured,
                        preserve_spatial: false,
                        show_spatial: false,
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
                                account_scope: None,
                                state: None,
                                id: (&plan.nodes[i]).into(),
                                placement: crate::provider::entity_placement(&plan.nodes[i]),
                            })
                            .collect(),
                        complete: r.complete,
                        dependency,
                        issues: r.issues.clone(),
                    })
                    .collect(),
                entity: {
                    let mut entity = crate::model::ArchitectureEntity::terraform(fact.into());
                    entity.represented_account = fact.represented_account.clone();
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
                role: crate::classification::classify_entity(fact),
            })
            .collect(),
        edges: plan
            .edges
            .iter()
            .map(|edge| edge.endpoints().into())
            .collect(),
    };
    crate::provider::aws::organization_policies::facts(plan, &mut graph);
    crate::provider::aws::encryption::facts(plan, &mut graph);
    graph.relationships =
        super::relationships::collect(&graph, &super::relationships::deployments(&graph.nodes));
    graph
}
