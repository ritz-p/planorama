use crate::model::{DiagnosticReason, EntityMode, Graph, SyntheticComponent, TerraformGraph};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/unit/semantic/components.rs"]
mod tests;

pub(super) fn infer(raw: &TerraformGraph, graph: &Graph) -> Vec<SyntheticComponent> {
    let mut counts = BTreeMap::new();
    for node in &raw.nodes {
        *counts.entry(node.address.as_str()).or_insert(0) += 1;
    }
    let visible: BTreeSet<_> = graph.nodes.iter().map(|n| n.address.as_str()).collect();
    let eligible = |index: usize, kind: &str| {
        let node = &raw.nodes[index];
        node.resource_type == kind
            && node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.deposed_key.is_none()
            && counts[node.address.as_str()] == 1
            && visible.contains(node.address.as_str())
    };
    let mut references = BTreeMap::<_, Vec<_>>::new();
    for reference in &raw.attributes {
        references
            .entry((reference.target, reference.attribute.as_str()))
            .or_default()
            .push(reference);
    }
    let parent = |index, attribute, kind| -> Option<usize> {
        let entries = references.get(&(index, attribute))?;
        let [reference] = entries.as_slice() else {
            return None;
        };
        let [source] = reference.sources.as_slice() else {
            return None;
        };
        (reference.complete
            && !reference
                .issues
                .contains(&DiagnosticReason::DynamicInstanceSelection)
            && eligible(*source, kind))
        .then_some(*source)
    };
    let mut listeners = BTreeMap::new();
    let mut groups: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if !eligible(index, "aws_lb_listener") {
            continue;
        }
        if let Some(anchor) = parent(index, "load_balancer_arn", "aws_lb") {
            listeners.insert(index, anchor);
            let members = groups.entry(anchor).or_default();
            members.insert(raw.nodes[anchor].address.clone());
            members.insert(node.address.clone());
        }
    }
    for (index, node) in raw.nodes.iter().enumerate() {
        if !eligible(index, "aws_lb_listener_rule") {
            continue;
        }
        if let Some(anchor) = parent(index, "listener_arn", "aws_lb_listener")
            .and_then(|listener| listeners.get(&listener))
        {
            groups
                .get_mut(anchor)
                .expect("listener group exists")
                .insert(node.address.clone());
        }
    }
    let mut components: Vec<_> = groups
        .into_iter()
        .map(|(anchor, members)| {
            let address = &raw.nodes[anchor].address;
            SyntheticComponent {
                id: format!("logical:load_balancer:{address}"),
                kind: "load_balancer".into(),
                label: format!("Load balancer: {address}"),
                members: members.into_iter().collect(),
            }
        })
        .collect();
    components.sort();
    components
}
