use crate::model::{AttributeReference, Diagnostic, DiagnosticReason as Reason, EntityMode, Graph};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/routes.rs"]
mod tests;

fn failure(raw: &Input, route: usize, attribute: &str, reason: Reason) -> Diagnostic {
    Diagnostic {
        deposed_key: raw.nodes[route].deposed_key.clone(),
        address: raw.nodes[route].address.clone(),
        attribute: attribute.into(),
        reason,
    }
}

fn endpoint(
    raw: &Input,
    route: usize,
    reference: &AttributeReference,
    kinds: &[&str],
) -> Result<usize, Diagnostic> {
    let reason = if let Some(reason) = reference.issues.iter().next() {
        Some(*reason)
    } else if reference.sources.is_empty() {
        Some(Reason::NoResourceReference)
    } else if !reference.complete {
        Some(Reason::PartialResourceProvenance)
    } else if reference.sources.len() != 1 {
        Some(Reason::MultipleMatchingInstances)
    } else if !reference.ids_only {
        Some(Reason::NonIdReference)
    } else if !reference.scalar_id_matches {
        Some(Reason::UnprovenEndpointValue)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(failure(raw, route, &reference.attribute, reason));
    }
    let index = reference.sources[0];
    let node = &raw.nodes[index];
    if index == route || !node.provider.is_aws() || !kinds.contains(&node.resource_type.as_str()) {
        return Err(failure(
            raw,
            route,
            &reference.attribute,
            Reason::EndpointTypeMismatch,
        ));
    }
    if node.deposed_key.is_some() {
        return Err(failure(
            raw,
            route,
            &reference.attribute,
            Reason::DeposedEndpoint,
        ));
    }
    Ok(index)
}

pub(super) fn owner(raw: &Input, route: usize) -> Result<usize, Diagnostic> {
    let reference = raw
        .attributes
        .iter()
        .find(|a| a.target == route && a.attribute == "route_table_id")
        .ok_or_else(|| failure(raw, route, "route_table_id", Reason::MissingAttribute))?;
    endpoint(raw, route, reference, &["aws_route_table"])
}

pub(super) fn target(raw: &Input, route: usize) -> Result<usize, Diagnostic> {
    let mut targets = raw.attributes.iter().filter(|a| {
        a.target == route
            && (a.attribute.ends_with("_id") || a.attribute.ends_with("_arn"))
            && !matches!(
                a.attribute.as_str(),
                "route_table_id" | "destination_prefix_list_id"
            )
    });
    let reference = targets
        .next()
        .ok_or_else(|| failure(raw, route, "route_target", Reason::MissingAttribute))?;
    if targets.next().is_some() {
        return Err(failure(
            raw,
            route,
            "route_target",
            Reason::AmbiguousRouteTarget,
        ));
    }
    let kinds = target_types(&reference.attribute).ok_or_else(|| {
        failure(
            raw,
            route,
            &reference.attribute,
            Reason::UnsupportedRouteTarget,
        )
    })?;
    if matches!(
        reference.attribute.as_str(),
        "vpc_endpoint_id" | "core_network_arn" | "odb_network_arn"
    ) {
        return if super::service_references::verified(raw, reference) {
            Ok(reference.sources[0])
        } else {
            Err(failure(
                raw,
                route,
                &reference.attribute,
                Reason::UnprovenEndpointValue,
            ))
        };
    }
    endpoint(raw, route, reference, kinds)
}

fn target_types(attribute: &str) -> Option<&'static [&'static str]> {
    Some(match attribute {
        "gateway_id" => &["aws_internet_gateway", "aws_vpn_gateway"],
        "nat_gateway_id" => &["aws_nat_gateway"],
        "transit_gateway_id" => &["aws_ec2_transit_gateway"],
        "vpc_peering_connection_id" => &["aws_vpc_peering_connection"],
        "network_interface_id" => &["aws_network_interface"],
        "egress_only_gateway_id" => &["aws_egress_only_internet_gateway"],
        "carrier_gateway_id" => &["aws_ec2_carrier_gateway"],
        "local_gateway_id" => &["aws_ec2_local_gateway"],
        "vpc_endpoint_id" => &["aws_vpc_endpoint"],
        "core_network_arn" => &["aws_networkmanager_core_network"],
        "odb_network_arn" => &["aws_odb_network"],
        _ => return None,
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if node.resource_type == "aws_route_table"
            && node.provider.is_aws()
            && node.mode == EntityMode::Managed
        {
            graph.nodes[index].routes.inline = !node.routes.entries.is_empty();
            for entry in &mut graph.nodes[index].routes.entries {
                if matches!(
                    entry.target_attribute.as_deref(),
                    Some("vpc_endpoint_id" | "core_network_arn" | "odb_network_arn")
                ) {
                    entry.target = None;
                    entry.target_kind = None;
                    continue;
                }
                let resolved = entry
                    .target
                    .as_ref()
                    .and_then(|id| raw.nodes.iter().find(|n| n.entity.provenance.contains(id)))
                    .filter(|n| {
                        n.provider.is_aws()
                            && n.deposed_key.is_none()
                            && entry
                                .target_attribute
                                .as_deref()
                                .and_then(target_types)
                                .is_some_and(|types| types.contains(&n.resource_type.as_str()))
                    });
                if let Some(target) = resolved {
                    entry.target_kind = Some(target.resource_type.clone());
                } else {
                    entry.target = None;
                }
            }
            if graph.nodes[index].routes.entries.len() == 1 {
                let refs = raw
                    .attributes
                    .iter()
                    .filter(|r| {
                        r.target == index
                            && matches!(
                                r.attribute.as_str(),
                                "route.vpc_endpoint_id"
                                    | "route.core_network_arn"
                                    | "route.odb_network_arn"
                            )
                            && super::service_references::verified(raw, r)
                    })
                    .collect::<Vec<_>>();
                if let [r] = refs.as_slice() {
                    if let [source] = r.sources.as_slice() {
                        let entry = &mut graph.nodes[index].routes.entries[0];
                        if entry.target_attribute.as_deref() != r.attribute.strip_prefix("route.") {
                            continue;
                        }
                        entry.target = raw.nodes[*source].entity.provenance.first().cloned();
                        entry.target_kind = Some(raw.nodes[*source].resource_type.clone());
                        entry.target_attribute =
                            r.attribute.strip_prefix("route.").map(str::to_owned);
                    }
                }
            }
        }
        if node.resource_type != "aws_route"
            || !node.provider.is_aws()
            || node.mode != EntityMode::Managed
        {
            continue;
        }
        let presentation = &mut graph.nodes[index].routes;
        if presentation.entries.is_empty() {
            presentation.entries.push(Default::default());
        }
        if let Ok(table) = owner(raw, index) {
            presentation.owner = raw.nodes[table].entity.provenance.first().cloned();
        }
        if let Ok(target) = target(raw, index) {
            presentation.entries[0].target = raw.nodes[target].entity.provenance.first().cloned();
            presentation.entries[0].target_kind = Some(raw.nodes[target].resource_type.clone());
            if presentation.owner.is_some() {
                graph.edges.retain(|e| {
                    !(e.from == target
                        && e.to == index
                        && e.kind == crate::model::EdgeKind::Dependency)
                });
                graph.edges.push(crate::model::Edge {
                    from: index,
                    to: target,
                    kind: crate::model::EdgeKind::Association,
                    change: Some(crate::model::EdgeChange {
                        directionality: crate::model::Directionality::Directed,
                        address: node.address.clone(),
                        previous_address: node.previous_address.clone(),
                        metadata: node.metadata.clone(),
                        action: node.action,
                    }),
                });
            }
        }
    }
}
