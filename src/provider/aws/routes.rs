use crate::model::{Diagnostic, DiagnosticReason as Reason};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/routes.rs"]
mod tests;

pub(super) fn endpoints(raw: &Input, route: usize) -> Result<(usize, usize), Diagnostic> {
    let failure = |attribute: &str, reason| Diagnostic {
        address: raw.nodes[route].address.clone(),
        attribute: attribute.into(),
        reason,
    };
    let mut targets = raw.attributes.iter().filter(|attribute| {
        attribute.target == route
            && (attribute.attribute.ends_with("_id") || attribute.attribute.ends_with("_arn"))
            && !matches!(
                attribute.attribute.as_str(),
                "route_table_id" | "destination_prefix_list_id"
            )
    });
    let target = targets
        .next()
        .ok_or_else(|| failure("route_target", Reason::MissingAttribute))?;
    if targets.next().is_some() {
        return Err(failure("route_target", Reason::AmbiguousRouteTarget));
    }
    let target_type = match target.attribute.as_str() {
        "gateway_id" => "aws_internet_gateway",
        "nat_gateway_id" => "aws_nat_gateway",
        "transit_gateway_id" => "aws_ec2_transit_gateway",
        "vpc_peering_connection_id" => "aws_vpc_peering_connection",
        _ => return Err(failure(&target.attribute, Reason::UnsupportedRouteTarget)),
    };
    let table = raw
        .attributes
        .iter()
        .find(|a| a.target == route && a.attribute == "route_table_id")
        .ok_or_else(|| failure("route_table_id", Reason::MissingAttribute))?;
    let mut endpoints = [0; 2];
    for (slot, reference, kind) in [(0, table, "aws_route_table"), (1, target, target_type)] {
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
            return Err(failure(&reference.attribute, reason));
        }
        let endpoint = super::associations::endpoint(raw, route, &reference.attribute, kind)
            .ok_or_else(|| failure(&reference.attribute, Reason::EndpointTypeMismatch))?;
        if raw.nodes[endpoint].deposed_key.is_some() {
            return Err(failure(&reference.attribute, Reason::DeposedEndpoint));
        }
        endpoints[slot] = endpoint;
    }
    Ok((endpoints[0], endpoints[1]))
}
