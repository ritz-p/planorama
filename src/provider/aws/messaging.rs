use crate::model::{
    Directionality, Edge, EdgeChange, EdgeKind, EntityMode, Graph, ResourceReference,
};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/messaging.rs"]
mod tests;

fn endpoint(raw: &Input, index: usize, attribute: &str) -> Option<usize> {
    let reference = raw
        .attributes
        .iter()
        .find(|r| r.target == index && r.attribute == attribute)?;
    let [source] = reference.sources.as_slice() else {
        return None;
    };
    let node = &raw.nodes[*source];
    (reference.complete
        && reference.instances_explicit
        && reference.issues.is_empty()
        && reference.arn_values_match
        && node.provider.is_aws()
        && node.deposed_key.is_none()
        && if attribute == "topic_arn" {
            node.resource_type == "aws_sns_topic"
        } else {
            reference.messaging_endpoint_verified
        })
    .then_some(*source)
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() || node.resource_type != "aws_sns_topic_subscription" {
            continue;
        }
        let supported = node.mode == EntityMode::Managed && node.deposed_key.is_none();
        let from = supported
            .then(|| endpoint(raw, index, "topic_arn"))
            .flatten();
        let to = supported
            .then(|| endpoint(raw, index, "endpoint"))
            .flatten();
        for (attribute, label, target) in [
            ("topic_arn", "Subscription topic", from),
            ("endpoint", "Subscription endpoint", to),
        ] {
            graph.nodes[index]
                .references
                .retain(|r| r.dependency || r.attribute != attribute);
            graph.nodes[index].references.push(ResourceReference {
                label,
                attribute: attribute.into(),
                targets: target
                    .into_iter()
                    .map(|i| crate::model::ReferenceTarget {
                        state: None,
                        id: raw.nodes[i].entity.provenance[0].clone(),
                        placement: raw.nodes[i].placement,
                    })
                    .collect(),
                complete: target.is_some(),
                dependency: false,
                issues: Default::default(),
            });
        }
        if let Some((from, to)) = from.zip(to) {
            graph.edges.push(Edge {
                from,
                to,
                kind: EdgeKind::Subscription,
                change: Some(EdgeChange {
                    directionality: Directionality::Directed,
                    address: node.address.clone(),
                    previous_address: node.previous_address.clone(),
                    metadata: node.metadata.clone(),
                    action: node.action,
                }),
            });
        }
    }
}
