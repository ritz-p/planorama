use crate::model::architecture::Architecture;
use crate::model::{
    Action, ArchitectureGraph, AwsAccountScope, EntityMode, Node, Placement, ReferenceKind,
    ReferenceTarget, ResourceReference, StateId, TerraformEntityId,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/organization_accounts.rs"]
mod tests;

fn member(node: &Node) -> bool {
    node.provider.is_aws()
        && node.resource_type == "aws_organizations_account"
        && node.mode == EntityMode::Managed
        && node.deposed_key.is_none()
        && node.action != Action::Delete
}

fn reference(
    kind: ReferenceKind,
    label: &'static str,
    targets: Vec<ReferenceTarget>,
) -> ResourceReference {
    ResourceReference {
        kind,
        preserve_spatial: false,
        show_spatial: true,
        label,
        attribute: "deployment_scope".into(),
        targets,
        complete: true,
        dependency: false,
        issues: BTreeSet::new(),
    }
}

pub(crate) fn link(all: &Architecture, selected: &mut Architecture) {
    for ArchitectureGraph(graph) in selected.states.values_mut() {
        for node in &mut graph.nodes {
            node.references.retain(|r| {
                !(matches!(
                    r.kind,
                    ReferenceKind::DeploymentScope | ReferenceKind::DeploymentMember
                ) || r.kind == ReferenceKind::Information && r.attribute == "deployment_scope")
            });
        }
    }
    let mut candidates = BTreeMap::<AwsAccountScope, usize>::new();
    for node in all
        .states
        .values()
        .flat_map(|graph| &graph.nodes)
        .filter(|node| member(node))
    {
        if let Some(account) = &node.entity.represented_account {
            *candidates.entry(account.clone()).or_default() += 1;
        }
    }
    if !selected
        .states
        .values()
        .flat_map(|graph| &graph.nodes)
        .any(member)
    {
        return;
    }
    let mut scopes = BTreeMap::<(StateId, AwsAccountScope), (usize, TerraformEntityId)>::new();
    for (state, graph) in &selected.states {
        let projection = crate::view::spatial::project(graph);
        let tree = crate::layout::ContainmentTree::new(&projection.graph);
        let groups = crate::view::scope::groups(&projection.graph, &tree);
        let mut visible = BTreeSet::new();
        for (scope, roots) in groups {
            if scope
                .as_ref()
                .and_then(|scope| scope.account.as_ref())
                .is_none()
            {
                continue;
            }
            let mut pending = roots;
            while let Some(index) = pending.pop() {
                visible.insert(projection.nodes[index]);
                pending.extend(tree.children[index].iter().copied());
            }
        }
        for index in visible {
            let node = &graph.nodes[index];
            if !node.provider.is_aws()
                || node.placement != Placement::Spatial
                || node.deposed_key.is_some()
                || node.action == Action::Delete
            {
                continue;
            }
            let Some(account) = node
                .entity
                .scope
                .as_ref()
                .and_then(|scope| scope.account.as_ref())
            else {
                continue;
            };
            let Some(id) = node.entity.provenance.first() else {
                continue;
            };
            let entry = scopes
                .entry((state.clone(), account.clone()))
                .or_insert((index, id.clone()));
            if id < &entry.1 {
                *entry = (index, id.clone());
            }
        }
    }
    let mut additions = Vec::new();
    for (state, graph) in &selected.states {
        for (index, node) in graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| member(node))
        {
            let mut targets = Vec::new();
            let label = match &node.entity.represented_account {
                None => "Deployment scope: member Account identity is unknown",
                Some(account) if candidates.get(account).copied().unwrap_or(0) != 1 => {
                    "Deployment scope: ambiguous member Account identity"
                }
                Some(account) => {
                    for ((scope_state, scope_account), (witness, id)) in &scopes {
                        if account != scope_account {
                            continue;
                        }
                        targets.push(ReferenceTarget {
                            account_scope: Some(account.clone()),
                            state: (scope_state != state).then(|| scope_state.clone()),
                            id: id.clone(),
                            placement: Placement::Spatial,
                        });
                        additions.push((
                            scope_state.clone(),
                            *witness,
                            reference(
                                ReferenceKind::DeploymentMember,
                                "Organizations member Account",
                                vec![ReferenceTarget {
                                    account_scope: None,
                                    state: (scope_state != state).then(|| state.clone()),
                                    id: node.entity.provenance[0].clone(),
                                    placement: node.placement,
                                }],
                            ),
                        ));
                    }
                    "Deployment scope: no visible verified match"
                }
            };
            additions.push((
                state.clone(),
                index,
                if targets.is_empty() {
                    reference(ReferenceKind::Information, label, targets)
                } else {
                    reference(
                        ReferenceKind::DeploymentScope,
                        "Verified AWS Account deployment scope",
                        targets,
                    )
                },
            ));
        }
    }
    for (state, index, reference) in additions {
        selected
            .states
            .get_mut(&state)
            .expect("selected state")
            .0
            .nodes[index]
            .references
            .push(reference);
    }
}
