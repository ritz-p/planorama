use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ec2_transit_gateway_multicast_domain", "transit_gateway_id") => Rule::new(
            "Multicast transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        (
            "aws_ec2_transit_gateway_multicast_domain_association"
            | "aws_ec2_transit_gateway_multicast_group_member"
            | "aws_ec2_transit_gateway_multicast_group_source",
            "transit_gateway_multicast_domain_id",
        ) => Rule::new(
            "Multicast domain",
            &[("aws_ec2_transit_gateway_multicast_domain", &["id"])],
            false,
            &[],
        ),
        (
            "aws_ec2_transit_gateway_multicast_domain_association",
            "transit_gateway_attachment_id",
        ) => Rule::new(
            "Multicast VPC attachment",
            &[("aws_ec2_transit_gateway_vpc_attachment", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_multicast_domain_association", "subnet_id") => Rule::new(
            "Multicast attachment subnet",
            &[("aws_subnet", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_multicast_group_member", "network_interface_id") => Rule::new(
            "Multicast member interface",
            &[("aws_network_interface", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_multicast_group_source", "network_interface_id") => Rule::new(
            "Multicast source interface",
            &[("aws_network_interface", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_transit_gateway_multicast_domain"
            | "aws_ec2_transit_gateway_multicast_domain_association"
            | "aws_ec2_transit_gateway_multicast_group_member"
            | "aws_ec2_transit_gateway_multicast_group_source"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, _path: &str) -> bool {
    let kind = e.nodes[r.target].resource_type.as_str();
    if kind == "aws_ec2_transit_gateway_multicast_domain" {
        return true;
    }
    let Some(domain) = e.configured_endpoint(
        r.target,
        "transit_gateway_multicast_domain_id",
        "aws_ec2_transit_gateway_multicast_domain",
        &["id"],
    ) else {
        return false;
    };
    if kind != "aws_ec2_transit_gateway_multicast_domain_association" {
        return e
            .text(r.target, "group_ip_address")
            .and_then(|ip| ip.parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_multicast())
            && e.configured_endpoint(
                r.target,
                "network_interface_id",
                "aws_network_interface",
                &["id"],
            )
            .is_some();
    }
    let Some(attachment) = e.configured_endpoint(
        r.target,
        "transit_gateway_attachment_id",
        "aws_ec2_transit_gateway_vpc_attachment",
        &["id"],
    ) else {
        return false;
    };
    let Some(subnet) = e.configured_endpoint(r.target, "subnet_id", "aws_subnet", &["id"]) else {
        return false;
    };
    let Some(gateway) = super::transit_routes::gateway(e, domain) else {
        return false;
    };
    super::transit_routes::attachment_on(e, attachment, gateway)
        && configured_links::network_context(e, attachment, &[("subnet_ids", "aws_subnet")])
            .zip(e.configured_endpoint(attachment, "vpc_id", "aws_vpc", &["id"]))
            .is_some_and(|(a, b)| a == b)
        && super::lambda_core::unique(e.refs, attachment, "subnet_ids")
            .is_some_and(|r| r.sources.contains(&subnet))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transit_multicast.rs"]
mod tests;
