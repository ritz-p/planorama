use crate::model::{AttributeReference, RegionScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwan.rs"]
mod tests;
const CORE: &str = "aws_networkmanager_core_network";
const VPC: &str = "aws_networkmanager_vpc_attachment";
const CONNECT: &str = "aws_networkmanager_connect_attachment";
const PEER: &str = "aws_networkmanager_connect_peer";
const PEERING: &str = "aws_networkmanager_transit_gateway_peering";
const TABLE: &str = "aws_networkmanager_transit_gateway_route_table_attachment";
const DX: &str = "aws_networkmanager_dx_gateway_attachment";
const VPN: &str = "aws_networkmanager_site_to_site_vpn_attachment";
const ACCEPT: &str = "aws_networkmanager_attachment_accepter";
const LABEL: &str = "aws_networkmanager_attachment_routing_policy_label";
const PREFIX: &str = "aws_networkmanager_prefix_list_association";
const POLICY: &str = "aws_networkmanager_core_network_policy_attachment";
pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        VPC | CONNECT | PEER | PEERING | TABLE | DX | VPN | ACCEPT | LABEL | PREFIX | POLICY
    )
}
fn direct_core(kind: &str) -> bool {
    matches!(
        kind,
        VPC | CONNECT | PEERING | DX | VPN | LABEL | PREFIX | POLICY
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if direct_core(kind) && path == "core_network_id" {
        return Some("Cloud WAN core network");
    }
    match (kind, path) {
        (PREFIX, "prefix_list_arn") => Some("Cloud WAN prefix list"),
        (LABEL, "attachment_id") => Some("Routing label attachment"),
        (CONNECT, "transport_attachment_id") => Some("Connect transport attachment"),
        (PEER, "connect_attachment_id") => Some("Peer Connect attachment"),
        (PEER, "subnet_arn") => Some("Tunnel-less peer subnet"),
        (DX, "direct_connect_gateway_arn") => Some("Cloud WAN Direct Connect gateway"),
        (VPN, "vpn_connection_arn") => Some("Cloud WAN VPN connection"),
        (PEERING, "transit_gateway_arn") => Some("Cloud WAN transit gateway"),
        (TABLE, "peering_id") => Some("Route table peering"),
        (TABLE, "transit_gateway_route_table_arn") => Some("Peering transit gateway route table"),
        (VPC, "vpc_arn") => Some("Cloud WAN attachment VPC"),
        (VPC, "subnet_arns") => Some("Cloud WAN attachment subnets"),
        (ACCEPT, "attachment_id") => Some("Accepted attachment"),
        _ => None,
    }
}
fn attachment_type(kind: &str) -> Option<&'static str> {
    match kind {
        VPC => Some("VPC"),
        CONNECT => Some("CONNECT"),
        DX => Some("DIRECT_CONNECT_GATEWAY"),
        VPN => Some("SITE_TO_SITE_VPN"),
        TABLE => Some("TRANSIT_GATEWAY_ROUTE_TABLE"),
        _ => None,
    }
}
fn attachment_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        CONNECT => &["id", "attachment_id"],
        VPC | DX | VPN | TABLE => &["id"],
        _ => &[],
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("core_network_id", CORE)
        | ("transport_attachment_id", VPC)
        | ("peering_id", PEERING)
        | ("transit_gateway_id", "aws_ec2_transit_gateway") => &["id"],
        ("connect_attachment_id", CONNECT) => &["id", "attachment_id"],
        ("attachment_id", _) => attachment_fields(source),
        ("prefix_list_arn", "aws_ec2_managed_prefix_list")
        | ("subnet_arn" | "subnet_arns", "aws_subnet")
        | ("vpc_arn", "aws_vpc")
        | ("direct_connect_gateway_arn", "aws_dx_gateway")
        | ("vpn_connection_arn", "aws_vpn_connection")
        | ("transit_gateway_arn", "aws_ec2_transit_gateway")
        | ("transit_gateway_route_table_arn", "aws_ec2_transit_gateway_route_table") => &["arn"],
        _ => &[],
    }
}
fn value<'a>(e: &[(&'a Value, &Value)], i: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(e[i].0, e[i].1, path)
}
fn known<'a>(e: &[(&'a Value, &Value)], i: usize, path: &str) -> Option<&'a str> {
    value(e, i, path)?.as_str().filter(|s| !s.is_empty())
}
fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
        value(e, r.target, path).is_some_and(|v| {
            if r.attribute == "subnet_arns" {
                v.is_array()
            } else {
                v.is_string()
            }
        })
    })
    .is_some()
}
fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
    path: &str,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, t, path)?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    basic(nodes, refs, e, r).then_some(*s)
}
fn core(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    if direct_core(&nodes[t].resource_type) {
        return endpoint(nodes, refs, e, t, "core_network_id");
    }
    if nodes[t].resource_type == TABLE {
        let p = endpoint(nodes, refs, e, t, "peering_id")?;
        return endpoint(nodes, refs, e, p, "core_network_id");
    }
    None
}
fn vpc(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<Vec<usize>> {
    if nodes[t].resource_type != VPC {
        return None;
    }
    let vpc = endpoint(nodes, refs, e, t, "vpc_arn")?;
    let subnets = super::lambda_core::unique(refs, t, "subnet_arns")?;
    if !basic(nodes, refs, e, subnets)
        || !subnets
            .sources
            .iter()
            .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(vpc))
    {
        return None;
    }
    let mut context = vec![vpc, core(nodes, refs, e, t)?];
    context.extend(&subnets.sources);
    Some(context)
}
fn transport(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<(Vec<usize>, RegionScope)> {
    if nodes[t].resource_type != CONNECT
        || e[t].0["options"].as_array().is_none_or(|a| a.len() != 1)
        || !matches!(known(e, t, "options.0.protocol"), Some("GRE" | "NO_ENCAP"))
    {
        return None;
    }
    let s = endpoint(nodes, refs, e, t, "transport_attachment_id")?;
    let parent = core(nodes, refs, e, t)?;
    if core(nodes, refs, e, s) != Some(parent) {
        return None;
    }
    let mut context = vpc(nodes, refs, e, s)?;
    context.extend([s, t]);
    Some((context, RegionScope(known(e, t, "edge_location")?.into())))
}
fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<(Vec<usize>, Option<RegionScope>)> {
    if !basic(nodes, refs, e, r) {
        return None;
    }
    let t = r.target;
    let kind = nodes[t].resource_type.as_str();
    let path = r.attribute.as_str();
    let mut context = Vec::new();
    let mut region = None;
    if path == "core_network_id" {
        return Some((context, region));
    }
    match (kind, path) {
        (VPC, "vpc_arn" | "subnet_arns") => context = vpc(nodes, refs, e, t)?,
        (CONNECT, "transport_attachment_id") => {
            let (c, r) = transport(nodes, refs, e, t)?;
            context = c;
            region = Some(r);
        }
        (PEER, "connect_attachment_id" | "subnet_arn") => {
            let attachment = endpoint(nodes, refs, e, t, "connect_attachment_id")?;
            let (c, edge_region) = transport(nodes, refs, e, attachment)?;
            if path == "subnet_arn"
                && (known(e, attachment, "options.0.protocol") != Some("NO_ENCAP")
                    || r.sources.is_empty())
            {
                return None;
            }
            if path == "subnet_arn"
                && !r
                    .sources
                    .iter()
                    .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(c[0]))
            {
                return None;
            }
            context = c;
            region = Some(edge_region);
        }
        (TABLE, "peering_id" | "transit_gateway_route_table_arn") => {
            let peering = endpoint(nodes, refs, e, t, "peering_id")?;
            let gateway = endpoint(nodes, refs, e, peering, "transit_gateway_arn")?;
            context.extend([peering, gateway, core(nodes, refs, e, peering)?]);
            if path == "transit_gateway_route_table_arn" {
                for &s in &r.sources {
                    if endpoint(nodes, refs, e, s, "transit_gateway_id") != Some(gateway) {
                        return None;
                    }
                }
            }
        }
        (LABEL, "attachment_id") => {
            let parent = core(nodes, refs, e, t)?;
            if !r
                .sources
                .iter()
                .all(|&s| core(nodes, refs, e, s) == Some(parent))
            {
                return None;
            }
            context.push(parent);
            for &source in &r.sources {
                if nodes[source].resource_type == TABLE {
                    context.push(endpoint(nodes, refs, e, source, "peering_id")?);
                }
            }
        }
        (ACCEPT, "attachment_id") => {
            let [s] = r.sources.as_slice() else {
                return None;
            };
            if attachment_type(&nodes[*s].resource_type) != known(e, t, "attachment_type") {
                return None;
            }
            context.push(core(nodes, refs, e, *s)?);
            if nodes[*s].resource_type == TABLE {
                context.push(endpoint(nodes, refs, e, *s, "peering_id")?);
            }
        }
        (DX, "direct_connect_gateway_arn") => {
            let a = value(e, t, "edge_locations")?.as_array()?;
            let set = a
                .iter()
                .map(|v| v.as_str().filter(|s| !s.is_empty()))
                .collect::<Option<BTreeSet<_>>>()?;
            if set.is_empty() || set.len() != a.len() {
                return None;
            }
            context.push(core(nodes, refs, e, t)?);
        }
        _ => context.push(core(nodes, refs, e, t)?),
    }
    context.sort_unstable();
    context.dedup();
    Some((context, region))
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).map(|_| proof(nodes, refs, e, r))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(result) = result {
            r.service_reference_verified = result.is_some();
            if let Some((context, region)) = result {
                r.endpoint_network_context = context;
                r.service_reference_region = region;
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let a = &raw.nodes[r.target];
    let mut regional = r.service_reference_region.as_ref();
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let b = &raw.nodes[s];
            if !super::iam_policies::same_account(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
            ) {
                return false;
            }
            let Some(scope) = b.entity.scope.as_ref() else {
                return false;
            };
            if scope.global {
                return true;
            }
            let Some(region) = scope.region.as_ref() else {
                return false;
            };
            if let Some(expected) = regional {
                expected == region
            } else {
                regional = Some(region);
                true
            }
        })
}
