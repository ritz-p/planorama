use crate::model::{DiagnosticReason, EntityMode, Graph, SyntheticComponent};
use crate::semantic::Input;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/components.rs"]
mod tests;

pub(super) fn infer(raw: &Input, graph: &Graph) -> Vec<SyntheticComponent> {
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
    let mut owners = BTreeMap::new();
    let mut groups: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if !eligible(index, "aws_lb_listener") {
            continue;
        }
        if let Some(anchor) = parent(index, "load_balancer_arn", "aws_lb") {
            listeners.insert(index, anchor);
            if references
                .get(&(index, "load_balancer_arn"))
                .is_some_and(|entries| entries.iter().all(|r| verified(r) && r.arn_values_match))
            {
                owners.insert(index, anchor);
            }
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
            if parent(index, "listener_arn", "aws_lb_listener")
                .is_some_and(|listener| owners.contains_key(&listener))
                && references
                    .get(&(index, "listener_arn"))
                    .is_some_and(|entries| {
                        entries.iter().all(|r| verified(r) && r.arn_values_match)
                    })
            {
                owners.insert(index, *anchor);
            }
            groups
                .get_mut(anchor)
                .expect("listener group exists")
                .insert(node.address.clone());
        }
    }
    for (target, anchor) in target_group_owners(raw, &owners, &eligible) {
        groups
            .get_mut(&anchor)
            .expect("owner group exists")
            .insert(raw.nodes[target].address.clone());
    }
    let mut components: Vec<_> = groups
        .into_iter()
        .map(|(anchor, members)| {
            let address = &raw.nodes[anchor].address;
            SyntheticComponent {
                entity: crate::model::ArchitectureEntity::synthetic(
                    "load_balancer",
                    address,
                    members
                        .iter()
                        .map(|address| crate::model::TerraformEntityId {
                            address: address.clone(),
                            deposed_key: None,
                        })
                        .collect(),
                ),
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

fn target_group_owners(
    raw: &Input,
    owners: &BTreeMap<usize, usize>,
    eligible: &impl Fn(usize, &str) -> bool,
) -> BTreeMap<usize, usize> {
    let mut candidates = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut blocked = BTreeSet::new();
    let mut supported = BTreeSet::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() || node.mode != EntityMode::Managed {
            continue;
        }
        let action = match node.resource_type.as_str() {
            "aws_lb_listener" => "default_action",
            "aws_lb_listener_rule" => "action",
            _ => continue,
        };
        let direct = format!("{action}.target_group_arn");
        let forward = format!("{action}.forward.target_group.arn");
        if raw.attributes.iter().any(|r| {
            r.target == index
                && (r.attribute == format!("{action}.forward")
                    || r.attribute == format!("{action}.forward.target_group"))
        }) {
            return BTreeMap::new();
        }
        let references: Vec<_> = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && (r.attribute == direct || r.attribute == forward))
            .collect();
        if references.is_empty()
            || references
                .iter()
                .any(|r| !verified(r) || !r.arn_values_match)
        {
            return BTreeMap::new();
        }
        let action_reference = raw
            .attributes
            .iter()
            .find(|r| r.target == index && r.attribute == action);
        if action_reference.is_none_or(|r| !verified(r)) {
            return BTreeMap::new();
        }
        for reference in references {
            if reference.sources.is_empty()
                || reference
                    .sources
                    .iter()
                    .any(|&source| !eligible(source, "aws_lb_target_group"))
            {
                return BTreeMap::new();
            }
            for &source in &reference.sources {
                supported.insert((index, reference.attribute.as_str(), source));
                if eligible(index, &node.resource_type) {
                    if let Some(&owner) = owners.get(&index) {
                        candidates.entry(source).or_default().insert(owner);
                        continue;
                    }
                }
                blocked.insert(source);
            }
        }
    }
    for reference in &raw.attributes {
        for &source in &reference.sources {
            if !candidates.contains_key(&source) {
                continue;
            }
            let node = &raw.nodes[reference.target];
            let action_use = supported.iter().any(|&(target, path, member)| {
                target == reference.target
                    && member == source
                    && (reference.attribute == path
                        || path
                            .strip_prefix(&reference.attribute)
                            .is_some_and(|suffix| suffix.starts_with('.')))
            });
            let attachment = node.provider.is_aws()
                && node.mode == EntityMode::Managed
                && node.resource_type == "aws_lb_target_group_attachment"
                && reference.attribute == "target_group_arn"
                && verified(reference)
                && reference.sources.len() == 1;
            if !action_use && !attachment {
                blocked.insert(source);
            }
        }
    }
    candidates
        .into_iter()
        .filter_map(|(target, owners)| {
            if blocked.contains(&target) || owners.len() != 1 {
                return None;
            }
            Some((target, *owners.first().expect("one owner")))
        })
        .collect()
}

fn verified(reference: &crate::model::AttributeReference) -> bool {
    reference.complete
        && reference.instances_explicit
        && !reference
            .issues
            .contains(&DiagnosticReason::DynamicInstanceSelection)
}
