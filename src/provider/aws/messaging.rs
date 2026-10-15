use crate::model::{
    Directionality, Edge, EdgeChange, EdgeKind, EntityMode, Graph, ResourceReference,
};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/messaging.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/event_sources.rs"]
mod event_source_tests;

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
        && node.provider.is_aws()
        && node.deposed_key.is_none()
        && match attribute {
            "topic_arn" => node.resource_type == "aws_sns_topic" && reference.arn_values_match,
            "event_source_arn" => {
                node.resource_type == "aws_sqs_queue" && reference.arn_values_match
            }
            "endpoint" => reference.messaging_endpoint_verified,
            "function_name" => {
                node.resource_type == "aws_lambda_function" && reference.messaging_endpoint_verified
            }
            _ => false,
        })
    .then_some(*source)
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() {
            continue;
        }
        let (kind, from_attribute, to_attribute, from_label, to_label) =
            match node.resource_type.as_str() {
                "aws_sns_topic_subscription" => (
                    EdgeKind::Subscription,
                    "topic_arn",
                    "endpoint",
                    "Subscription topic",
                    "Subscription endpoint",
                ),
                "aws_lambda_event_source_mapping" => (
                    EdgeKind::EventSource,
                    "event_source_arn",
                    "function_name",
                    "Event source",
                    "Consumer function",
                ),
                _ => continue,
            };
        let supported = node.mode == EntityMode::Managed && node.deposed_key.is_none();
        let from = supported
            .then(|| endpoint(raw, index, from_attribute))
            .flatten();
        let to = supported
            .then(|| endpoint(raw, index, to_attribute))
            .flatten();
        for (attribute, label, target) in [
            (from_attribute, from_label, from),
            (to_attribute, to_label, to),
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
                kind,
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
