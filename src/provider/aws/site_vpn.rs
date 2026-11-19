use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_customer_gateway", "certificate_arn") => Rule::new(
            "Gateway certificate",
            &[("aws_acm_certificate", &["arn"])],
            false,
            &[],
        ),
        ("aws_vpn_concentrator", "transit_gateway_id") => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_vpn_connection", "customer_gateway_id") => Rule::new(
            "Customer gateway",
            &[("aws_customer_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_vpn_connection", "transit_gateway_id") => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_vpn_connection", "vpn_gateway_id") => Rule::new(
            "Virtual private gateway",
            &[("aws_vpn_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_vpn_connection", "vpn_concentrator_id") => Rule::new(
            "VPN concentrator",
            &[("aws_vpn_concentrator", &["vpn_concentrator_id"])],
            false,
            &[],
        ),
        ("aws_vpn_connection", "transport_transit_gateway_attachment_id") => Rule::new(
            "Private transport attachment",
            &[("aws_ec2_transit_gateway_dx_gateway_attachment", &["id"])],
            false,
            &[],
        )
        .data_source(),
        ("aws_vpn_connection", "tunnel1_log_options.cloudwatch_log_options.log_group_arn") => {
            Rule::new(
                "Tunnel log group",
                &[("aws_cloudwatch_log_group", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_vpn_connection", "tunnel1_log_options.cloudwatch_log_options.bgp_log_group_arn") => {
            Rule::new(
                "BGP log group",
                &[("aws_cloudwatch_log_group", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_vpn_connection", "tunnel2_log_options.cloudwatch_log_options.log_group_arn") => {
            Rule::new(
                "Tunnel log group",
                &[("aws_cloudwatch_log_group", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_vpn_connection", "tunnel2_log_options.cloudwatch_log_options.bgp_log_group_arn") => {
            Rule::new(
                "BGP log group",
                &[("aws_cloudwatch_log_group", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_vpn_connection_route", "vpn_connection_id") => Rule::new(
            "Static VPN connection",
            &[("aws_vpn_connection", &["id"])],
            false,
            &[],
        ),
        ("aws_vpn_gateway" | "aws_vpn_gateway_attachment", "vpc_id") => {
            Rule::new("Gateway VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_vpn_gateway_attachment" | "aws_vpn_gateway_route_propagation", "vpn_gateway_id") => {
            Rule::new(
                "Virtual private gateway",
                &[("aws_vpn_gateway", &["id"])],
                false,
                &[],
            )
        }
        ("aws_vpn_gateway_route_propagation", "route_table_id") => Rule::new(
            "VPC route table",
            &[("aws_route_table", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_customer_gateway"
            | "aws_vpn_concentrator"
            | "aws_vpn_connection"
            | "aws_vpn_connection_route"
            | "aws_vpn_gateway"
            | "aws_vpn_gateway_attachment"
            | "aws_vpn_gateway_route_propagation"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let kind = e.nodes[r.target].resource_type.as_str();
    if kind == "aws_vpn_gateway_route_propagation" {
        let Some(gateway) =
            e.configured_endpoint(r.target, "vpn_gateway_id", "aws_vpn_gateway", &["id"])
        else {
            return false;
        };
        let Some(table) =
            e.configured_endpoint(r.target, "route_table_id", "aws_route_table", &["id"])
        else {
            return false;
        };
        return gateway_vpc(e, gateway)
            .zip(super::lambda_core::parent(e.nodes, e.refs, e.values, table))
            .is_some_and(|(a, b)| a == b);
    }
    match (kind, r.attribute.as_str()) {
        ("aws_vpn_connection_route", "vpn_connection_id") => {
            e.value(source, "static_routes_only")
                .and_then(Value::as_bool)
                == Some(true)
        }
        ("aws_vpn_connection", "vpn_gateway_id" | "transit_gateway_id" | "vpn_concentrator_id") => {
            [
                "vpn_gateway_id",
                "transit_gateway_id",
                "vpn_concentrator_id",
            ]
            .iter()
            .all(|&k| k == path || e.absent(r.target, k))
        }
        (_, "transport_transit_gateway_attachment_id") => {
            e.text(r.target, "outside_ip_address_type") == Some("PrivateIpv4")
        }
        (_, p) if p.ends_with("log_group_arn") => {
            path.strip_suffix("group_arn").is_some_and(|base| {
                e.value(r.target, &format!("{base}enabled"))
                    .and_then(Value::as_bool)
                    == Some(true)
            })
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/site_vpn.rs"]
mod tests;

fn gateway_vpc(e: &Evidence<'_>, gateway: usize) -> Option<usize> {
    let mut parents = std::collections::BTreeSet::new();
    if e.has_argument(gateway, "vpc_id") {
        parents.insert(e.configured_endpoint(gateway, "vpc_id", "aws_vpc", &["id"])?);
    }
    for r in e.refs.iter().filter(|r| {
        r.attribute == "vpn_gateway_id"
            && r.sources.contains(&gateway)
            && e.nodes[r.target].resource_type == "aws_vpn_gateway_attachment"
    }) {
        if e.configured_endpoint(r.target, "vpn_gateway_id", "aws_vpn_gateway", &["id"])
            != Some(gateway)
        {
            return None;
        }
        parents.insert(e.configured_endpoint(r.target, "vpc_id", "aws_vpc", &["id"])?);
    }
    let parent = parents.pop_first()?;
    (parents.is_empty() && e.optional_vpc_matches(gateway, parent)).then_some(parent)
}

pub(super) fn present(raw: &Input, graph: &mut crate::model::Graph) {
    use crate::model::{Edge, EdgeKind};
    use crate::semantic::relationships;
    let mut attachments =
        std::collections::BTreeMap::<usize, std::collections::BTreeMap<usize, Vec<_>>>::new();
    let endpoint = |target, path| {
        super::lambda_core::unique(&raw.attributes, target, path)
            .filter(|r| super::service_references::verified(raw, r))
            .and_then(|r| match r.sources.as_slice() {
                [one] => Some(*one),
                _ => None,
            })
    };
    for (i, node) in raw.nodes.iter().enumerate() {
        let (gateway, vpc) = match node.resource_type.as_str() {
            "aws_vpn_gateway" => (Some(i), endpoint(i, "vpc_id")),
            "aws_vpn_gateway_attachment" => (endpoint(i, "vpn_gateway_id"), endpoint(i, "vpc_id")),
            _ => continue,
        };
        if let Some((gateway, vpc)) = gateway.zip(vpc) {
            attachments
                .entry(gateway)
                .or_default()
                .entry(vpc)
                .or_default()
                .push(i);
        }
    }
    for (gateway, mut candidates) in attachments {
        if candidates.len() != 1 {
            continue;
        }
        let Some((vpc, proof)) = candidates.pop_first() else {
            continue;
        };
        let mut evidence = Vec::new();
        for i in proof {
            evidence.push(relationships::reference(graph, vpc, i));
            if i != gateway {
                evidence.push(relationships::reference(graph, gateway, i));
            }
        }
        relationships::record(graph, vpc, gateway, EdgeKind::Containment, evidence);
        graph
            .edges
            .retain(|edge| edge.endpoints() != (vpc, gateway) || edge.kind != EdgeKind::Dependency);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((vpc, gateway))
        });
    }
}
