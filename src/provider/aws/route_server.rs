use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/route_server.rs"]
mod tests;

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (kind, path) {
        (
            "aws_vpc_route_server_endpoint"
            | "aws_vpc_route_server_propagation"
            | "aws_vpc_route_server_vpc_association",
            "route_server_id",
        ) => Some(("aws_vpc_route_server", "route_server_id", "Route server")),
        ("aws_vpc_route_server_endpoint", "subnet_id")
        | ("aws_vpc_route_server_propagation", _) => Some(("aws_subnet", "id", "Endpoint subnet")),
        ("aws_vpc_route_server_peer", "route_server_endpoint_id") => Some((
            "aws_vpc_route_server_endpoint",
            "route_server_endpoint_id",
            "BGP peer endpoint",
        )),
        ("aws_vpc_route_server_propagation", "route_table_id") => {
            Some(("aws_route_table", "id", "Propagation table"))
        }
        ("aws_vpc_route_server_vpc_association", "vpc_id") => {
            Some(("aws_vpc", "id", "Associated VPC"))
        }
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}
fn reference<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    (found.next().is_none() && r.service_reference_verified).then_some(r)
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::service_evidence::annotate_scalar(
        nodes,
        refs,
        evidence,
        label,
        |kind, path, source, field| rule(kind, path).is_some_and(|r| r.0 == source && r.1 == field),
    );
    let valid: Vec<_> = refs
        .iter()
        .map(|r| {
            if !r.service_reference_verified
                || label(&nodes[r.target].resource_type, &r.attribute).is_none()
            {
                return r.service_reference_verified;
            }
            let known = |i, p| super::service_evidence::known(evidence, i, p);
            match (nodes[r.target].resource_type.as_str(), r.attribute.as_str()) {
                ("aws_vpc_route_server_endpoint", "subnet_id") => {
                    let Some(server) = reference(refs, r.target, "route_server_id") else {
                        return false;
                    };
                    let member =
                        if nodes[r.target].resource_type == "aws_vpc_route_server_propagation" {
                            let Some(table) = reference(refs, r.target, "route_table_id") else {
                                return false;
                            };
                            table.sources[0]
                        } else {
                            r.sources[0]
                        };
                    let Some(vpc) = known(member, "vpc_id") else {
                        return false;
                    };
                    if nodes[r.target].resource_type == "aws_vpc_route_server_endpoint"
                        && known(r.target, "vpc_id") != Some(vpc)
                    {
                        return false;
                    }
                    let associations: Vec<_> = nodes
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| n.resource_type == "aws_vpc_route_server_vpc_association")
                        .filter_map(|(i, _)| {
                            let parent = reference(refs, i, "route_server_id")?;
                            let network = reference(refs, i, "vpc_id")?;
                            (parent.sources == server.sources).then_some(network.sources[0])
                        })
                        .collect();
                    associations.len() == 1 && known(associations[0], "id") == Some(vpc)
                }
                ("aws_vpc_route_server_vpc_association", _) => {
                    reference(refs, r.target, "route_server_id").is_some()
                        && reference(refs, r.target, "vpc_id").is_some()
                }
                _ => true,
            }
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(valid) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = valid;
        }
    }
}
pub(super) fn association(raw: &Input, index: usize) -> Option<(usize, usize)> {
    let field = match raw.nodes[index].resource_type.as_str() {
        "aws_vpc_route_server_propagation" => "route_table_id",
        "aws_vpc_route_server_vpc_association" => "vpc_id",
        _ => return None,
    };
    let get = |p| {
        let r = reference(&raw.attributes, index, p)?;
        super::service_references::verified(raw, r).then_some(r.sources[0])
    };
    Some((get("route_server_id")?, get(field)?))
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if !super::apprunner::bindings(raw, r) {
        return false;
    }
    if !matches!(
        (
            raw.nodes[r.target].resource_type.as_str(),
            r.attribute.as_str()
        ),
        ("aws_vpc_route_server_endpoint", "subnet_id") | ("aws_vpc_route_server_propagation", _)
    ) {
        return true;
    }
    let Some(server) = reference(&raw.attributes, r.target, "route_server_id") else {
        return false;
    };
    if !super::apprunner::bindings(raw, server) {
        return false;
    }
    let candidates: Vec<_> = raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_vpc_route_server_vpc_association")
        .filter_map(|(index, _)| {
            let parent = reference(&raw.attributes, index, "route_server_id")?;
            let vpc = reference(&raw.attributes, index, "vpc_id")?;
            (parent.sources == server.sources).then_some((parent, vpc))
        })
        .collect();
    matches!(candidates.as_slice(), [(parent, vpc)] if super::apprunner::bindings(raw, parent) && super::apprunner::bindings(raw, vpc))
}
