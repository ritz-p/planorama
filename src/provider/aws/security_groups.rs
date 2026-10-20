use crate::model::{AttributeReference, DiagnosticReason, EdgeKind, EntityMode, Graph};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/security_groups.rs"]
mod tests;

pub(super) fn attribute(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "aws_instance" | "aws_db_instance" | "aws_rds_cluster" | "aws_db_proxy" => {
            Some("vpc_security_group_ids")
        }
        "aws_lb" | "aws_efs_mount_target" => Some("security_groups"),
        "aws_ecs_service" => Some("network_configuration.security_groups"),
        "aws_vpc_endpoint" | "aws_elasticache_serverless_cache" => Some("security_group_ids"),
        "aws_lambda_function" | "aws_eks_cluster" => Some("vpc_config.security_group_ids"),
        "aws_opensearch_domain" => Some("vpc_options.security_group_ids"),
        _ => None,
    }
}

pub(super) fn failure(raw: &Input, reference: &AttributeReference) -> Option<DiagnosticReason> {
    let node = &raw.nodes[reference.target];
    if super::network::endpoint_type(&node.resource_type, &reference.attribute).is_some()
        && !reference.service_reference_verified
    {
        return Some(DiagnosticReason::UnprovenEndpointValue);
    }
    if node.resource_type == "aws_efs_mount_target"
        && (node.action == crate::model::Action::Delete
            || node.deposed_key.is_some()
            || reference.sources.iter().any(|&source| {
                raw.nodes[source].mode != EntityMode::Managed
                    || raw.nodes[source].action == crate::model::Action::Delete
                    || raw.nodes[source].deposed_key.is_some()
            }))
    {
        return Some(DiagnosticReason::UnprovenEndpointValue);
    }
    if !reference.complete || reference.sources.is_empty() {
        return Some(if reference.sources.is_empty() {
            DiagnosticReason::NoResourceReference
        } else {
            DiagnosticReason::PartialResourceProvenance
        });
    }
    for reason in [
        DiagnosticReason::DynamicInstanceSelection,
        DiagnosticReason::MultipleMatchingInstances,
    ] {
        if reference.issues.contains(&reason) {
            return Some(reason);
        }
    }
    reference
        .sources
        .iter()
        .any(|&source| {
            source == reference.target
                || raw.nodes[source].resource_type != "aws_security_group"
                || !raw.nodes[source].provider.is_aws()
        })
        .then_some(DiagnosticReason::EndpointTypeMismatch)
        .or_else(|| {
            (!reference.collection_ids_complete)
                .then_some(DiagnosticReason::PartialResourceProvenance)
        })
}

pub(super) fn infer(raw: &Input, mut graph: Graph) -> Graph {
    for reference in &raw.attributes {
        let node = &raw.nodes[reference.target];
        if node.mode != EntityMode::Managed
            || !node.provider.is_aws()
            || attribute(&node.resource_type) != Some(reference.attribute.as_str())
            || failure(raw, reference).is_some()
        {
            continue;
        }
        let mut inferred = Vec::new();
        for edge in &mut graph.edges {
            if edge.to == reference.target
                && edge.kind == EdgeKind::Dependency
                && reference.sources.contains(&edge.from)
            {
                edge.kind = EdgeKind::Connection;
                inferred.push(edge.endpoints());
            }
        }
        for (from, to) in inferred {
            crate::semantic::relationships::record_reference(
                &mut graph,
                from,
                to,
                EdgeKind::Connection,
            );
        }
    }
    graph
}
