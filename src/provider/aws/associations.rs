use crate::model::{Edge, EdgeChange, EdgeKind, EntityMode, Graph, ResourceRole};
use crate::semantic::Input;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/associations.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/ebs.rs"]
mod ebs_tests;

pub(super) fn rule(resource_type: &str) -> Option<[(&'static str, &'static str); 2]> {
    let resource_type = super::aliases::canonical(resource_type);
    match resource_type {
        "aws_vpc_security_group_vpc_association" => Some([
            ("security_group_id", "aws_security_group"),
            ("vpc_id", "aws_vpc"),
        ]),
        "aws_vpc_endpoint_connection_accepter" => Some([
            ("vpc_endpoint_id", "aws_vpc_endpoint"),
            ("vpc_endpoint_service_id", "aws_vpc_endpoint_service"),
        ]),
        "aws_vpc_endpoint_subnet_association" => Some([
            ("vpc_endpoint_id", "aws_vpc_endpoint"),
            ("subnet_id", "aws_subnet"),
        ]),
        "aws_vpc_endpoint_security_group_association" => Some([
            ("vpc_endpoint_id", "aws_vpc_endpoint"),
            ("security_group_id", "aws_security_group"),
        ]),
        "aws_eip_association" => Some([
            ("allocation_id", "aws_eip"),
            ("instance_id", "aws_instance"),
        ]),
        "aws_nat_gateway_eip_association" => Some([
            ("nat_gateway_id", "aws_nat_gateway"),
            ("allocation_id", "aws_eip"),
        ]),
        "aws_internet_gateway_attachment" => Some([
            ("internet_gateway_id", "aws_internet_gateway"),
            ("vpc_id", "aws_vpc"),
        ]),
        "aws_wafv2_web_acl_association" => Some([
            ("web_acl_arn", "aws_wafv2_web_acl"),
            ("resource_arn", "aws_lb"),
        ]),
        "aws_volume_attachment" => Some([
            ("instance_id", "aws_instance"),
            ("volume_id", "aws_ebs_volume"),
        ]),
        "aws_route_table_association" => Some([
            ("subnet_id", "aws_subnet"),
            ("route_table_id", "aws_route_table"),
        ]),
        "aws_main_route_table_association" => {
            Some([("vpc_id", "aws_vpc"), ("route_table_id", "aws_route_table")])
        }
        "aws_lb_target_group_attachment" => Some([
            ("target_group_arn", "aws_lb_target_group"),
            ("target_id", "aws_instance"),
        ]),
        "aws_lb_listener_certificate" => Some([
            ("listener_arn", "aws_lb_listener"),
            ("certificate_arn", "aws_acm_certificate"),
        ]),
        "aws_vpc_endpoint_route_table_association" => Some([
            ("vpc_endpoint_id", "aws_vpc_endpoint"),
            ("route_table_id", "aws_route_table"),
        ]),
        _ => None,
    }
}

pub(super) fn lower(raw: &Input) -> Graph {
    let mut incident_edges = vec![Vec::new(); raw.nodes.len()];
    for edge in &raw.edges {
        incident_edges[edge.from].push(edge);
        if edge.from != edge.to {
            incident_edges[edge.to].push(edge);
        }
    }
    let mut replacements = BTreeMap::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if node.role != ResourceRole::Association
            || !node.provider.is_aws()
            || node.mode != EntityMode::Managed
            || node.deposed_key.is_some()
        {
            continue;
        }
        let endpoints = if super::aliases::node_type(node) == "aws_lb_target_group_attachment" {
            super::elbv2::association(raw, index)
        } else if super::aliases::node_type(node) == "aws_lb_listener_certificate" {
            super::elbv2_tls::association(raw, index)
        } else if node.resource_type == "aws_vpc_security_group_vpc_association" {
            super::security_group_rules::association(raw, index)
        } else if matches!(
            node.resource_type.as_str(),
            "aws_vpc_endpoint_connection_accepter"
                | "aws_vpc_endpoint_subnet_association"
                | "aws_vpc_endpoint_route_table_association"
                | "aws_vpc_endpoint_security_group_association"
        ) {
            super::privatelink::association(raw, index)
        } else if matches!(
            node.resource_type.as_str(),
            "aws_eip_association"
                | "aws_nat_gateway_eip_association"
                | "aws_internet_gateway_attachment"
        ) {
            super::vpc_gateways::association(raw, index)
        } else if matches!(
            node.resource_type.as_str(),
            "aws_route_table_association" | "aws_main_route_table_association"
        ) {
            super::vpc_routes::association(raw, index)
        } else {
            rule(&node.resource_type).and_then(
                |[(from_attribute, from_type), (to_attribute, to_type)]| {
                    endpoint(raw, index, from_attribute, from_type).zip(endpoint(
                        raw,
                        index,
                        to_attribute,
                        to_type,
                    ))
                },
            )
        };
        if let Some((from, to)) = endpoints {
            let incident = &incident_edges[index];
            if can_lower(index, from, to, incident) {
                replacements.insert(
                    index,
                    Edge {
                        from,
                        to,
                        kind: EdgeKind::Association,
                        change: Some(EdgeChange {
                            directionality: crate::model::Directionality::Undirected,
                            address: node.address.clone(),
                            previous_address: node.previous_address.clone(),
                            metadata: node.metadata.clone(),
                            action: node.action,
                        }),
                    },
                );
            }
        }
    }
    if replacements.is_empty() {
        return raw.graph.clone();
    }
    let mut indices = vec![None; raw.nodes.len()];
    let mut nodes = Vec::new();
    for (index, node) in raw.nodes.iter().enumerate() {
        if !replacements.contains_key(&index) {
            indices[index] = Some(nodes.len());
            nodes.push(node.clone());
        }
    }
    let mut edges: Vec<_> = raw
        .edges
        .iter()
        .chain(replacements.values())
        .filter_map(|edge| {
            indices[edge.from]
                .zip(indices[edge.to])
                .map(|(from, to)| Edge {
                    from,
                    to,
                    ..edge.clone()
                })
        })
        .collect();
    edges.sort();
    edges.dedup();
    Graph {
        coverage: Default::default(),
        relationships: raw.relationships.clone(),
        components: raw.components.clone(),
        checks: raw.checks.clone(),
        status: raw.status,
        nodes,
        edges,
    }
}

pub(super) fn can_lower(index: usize, from: usize, to: usize, incident: &[&Edge]) -> bool {
    incident.len() == 2
        && incident.iter().all(|e| {
            e.to == index && e.kind == EdgeKind::Dependency && (e.from == from || e.from == to)
        })
        && incident.iter().any(|e| e.from == from)
        && incident.iter().any(|e| e.from == to)
}

pub(super) fn endpoint(
    raw: &Input,
    target: usize,
    attribute: &str,
    resource_type: &str,
) -> Option<usize> {
    let reference = raw
        .attributes
        .iter()
        .find(|a| a.target == target && a.attribute == attribute)?;
    if matches!(
        raw.nodes[target].resource_type.as_str(),
        "aws_volume_attachment" | "aws_wafv2_web_acl_association"
    ) && !reference.service_reference_verified
    {
        return None;
    }
    match reference.sources.as_slice() {
        [source]
            if reference.complete
                && !reference
                    .issues
                    .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
                && *source != target
                && endpoint_type_matches(
                    &raw.nodes[target].resource_type,
                    attribute,
                    resource_type,
                    &raw.nodes[*source].resource_type,
                ) =>
        {
            raw.nodes[*source].provider.is_aws().then_some(*source)
        }
        _ => None,
    }
}

pub(super) fn endpoint_type_matches(
    resource: &str,
    attribute: &str,
    expected: &str,
    actual: &str,
) -> bool {
    super::aliases::canonical(actual) == super::aliases::canonical(expected)
        || (super::aliases::canonical(resource) == "aws_lb_listener_certificate"
            && attribute == "certificate_arn"
            && actual == "aws_iam_server_certificate")
        || (super::aliases::canonical(resource) == "aws_lb_target_group_attachment"
            && attribute == "target_id"
            && matches!(
                super::aliases::canonical(actual),
                "aws_lambda_function" | "aws_lb"
            ))
        || (resource == "aws_route_table_association"
            && attribute == "gateway_id"
            && actual == "aws_vpn_gateway")
        || (resource == "aws_wafv2_web_acl_association"
            && attribute == "resource_arn"
            && actual == "aws_api_gateway_stage")
}
