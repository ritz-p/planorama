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

#[cfg(test)]
#[path = "../../../tests/unit/semantic/event_targets.rs"]
mod event_target_tests;

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
                matches!(
                    node.resource_type.as_str(),
                    "aws_sqs_queue" | "aws_kinesis_stream"
                ) && reference.arn_values_match
                    && (node.resource_type != "aws_kinesis_stream"
                        || (reference.source_attributes.len() == 1
                            && reference.source_attributes.contains("arn")))
            }
            "endpoint" => reference.messaging_endpoint_verified,
            "rule" => {
                node.resource_type == "aws_cloudwatch_event_rule"
                    && reference.messaging_endpoint_verified
            }
            "arn" => {
                matches!(
                    node.resource_type.as_str(),
                    "aws_sqs_queue" | "aws_lambda_function"
                ) && reference.arn_values_match
            }
            "event_bus_name" => {
                node.resource_type == "aws_cloudwatch_event_bus"
                    && (reference.scalar_name_matches || reference.arn_values_match)
            }
            "function_name" => {
                node.resource_type == "aws_lambda_function" && reference.messaging_endpoint_verified
            }
            _ => false,
        })
    .then_some(*source)
}

pub(super) fn diagnostics(raw: &Input) -> Vec<crate::model::Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.provider.is_aws()
                && node.resource_type == "aws_lambda_event_source_mapping"
                && node.mode == EntityMode::Managed
                && node.deposed_key.is_none()
        })
        .flat_map(|(index, node)| {
            ["event_source_arn", "function_name"]
                .into_iter()
                .filter(move |attribute| endpoint(raw, index, attribute).is_none())
                .map(move |attribute| crate::model::Diagnostic {
                    address: node.address.clone(),
                    deposed_key: node.deposed_key.clone(),
                    attribute: attribute.into(),
                    reason: crate::model::DiagnosticReason::UnprovenEndpointValue,
                })
        })
        .collect()
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
                "aws_cloudwatch_event_target" => (
                    EdgeKind::EventTarget,
                    "rule",
                    "arn",
                    "Event rule",
                    "Event target",
                ),
                _ => continue,
            };
        let supported = node.mode == EntityMode::Managed && node.deposed_key.is_none();
        let from = supported
            .then(|| endpoint(raw, index, from_attribute))
            .flatten()
            .filter(|&rule| kind != EdgeKind::EventTarget || event_target_scope(raw, index, rule));
        let to = supported
            .then(|| endpoint(raw, index, to_attribute))
            .flatten();
        let bus = (supported && kind == EdgeKind::EventTarget)
            .then(|| endpoint(raw, index, "event_bus_name"))
            .flatten();
        for (attribute, label, target) in [
            (from_attribute, from_label, from),
            (to_attribute, to_label, to),
        ]
        .into_iter()
        .chain((kind == EdgeKind::EventTarget).then_some(("event_bus_name", "Event bus", bus)))
        {
            graph.nodes[index]
                .references
                .retain(|r| r.dependency || r.attribute != attribute);
            graph.nodes[index].references.push(ResourceReference {
                kind: crate::model::ReferenceKind::Configured,
                show_spatial: false,
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

fn event_target_scope(raw: &Input, target: usize, rule: usize) -> bool {
    let same_scope = |other: usize| {
        let source = &raw.nodes[target];
        let node = &raw.nodes[other];
        source.entity.scope == node.entity.scope
            && source.entity.scope.as_ref().is_some_and(|scope| {
                scope.region.is_some()
                    && (scope.account.is_some()
                        || source
                            .provider_configuration
                            .as_ref()
                            .is_some_and(|configuration| {
                                !configuration.account_conflict
                                    && node.provider_configuration.as_ref() == Some(configuration)
                            }))
            })
    };
    same_scope(rule)
        && raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == "event_bus_name")
            .all(|r| matches!(r.sources.as_slice(), [bus] if same_scope(*bus)))
}
