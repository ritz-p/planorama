use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_ec2_transit_gateway_route_table"
            | "aws_ec2_transit_gateway_default_route_table_association"
            | "aws_ec2_transit_gateway_default_route_table_propagation",
            "transit_gateway_id",
        ) => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        (
            "aws_ec2_transit_gateway_default_route_table_association"
            | "aws_ec2_transit_gateway_default_route_table_propagation"
            | "aws_ec2_transit_gateway_route"
            | "aws_ec2_transit_gateway_prefix_list_reference"
            | "aws_ec2_transit_gateway_route_table_association"
            | "aws_ec2_transit_gateway_route_table_propagation",
            "transit_gateway_route_table_id",
        ) => Rule::new(
            "Transit route table",
            &[("aws_ec2_transit_gateway_route_table", &["id"])],
            false,
            &[],
        ),
        (
            "aws_ec2_transit_gateway_route"
            | "aws_ec2_transit_gateway_prefix_list_reference"
            | "aws_ec2_transit_gateway_route_table_association"
            | "aws_ec2_transit_gateway_route_table_propagation",
            "transit_gateway_attachment_id",
        ) => Rule::new(
            "Route attachment",
            &[
                ("aws_ec2_transit_gateway_vpc_attachment", &["id"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["id"]),
                ("aws_ec2_transit_gateway_connect", &["id"]),
                ("aws_vpn_connection", &["transit_gateway_attachment_id"]),
                ("aws_vpn_concentrator", &["transit_gateway_attachment_id"]),
            ],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_prefix_list_reference", "prefix_list_id") => Rule::new(
            "Managed prefix list",
            &[("aws_ec2_managed_prefix_list", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_transit_gateway_route_table"
            | "aws_ec2_transit_gateway_default_route_table_association"
            | "aws_ec2_transit_gateway_default_route_table_propagation"
            | "aws_ec2_transit_gateway_route"
            | "aws_ec2_transit_gateway_prefix_list_reference"
            | "aws_ec2_transit_gateway_route_table_association"
            | "aws_ec2_transit_gateway_route_table_propagation"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    let kind = e.nodes[r.target].resource_type.as_str();
    if matches!(
        kind,
        "aws_ec2_transit_gateway_default_route_table_association"
            | "aws_ec2_transit_gateway_default_route_table_propagation"
    ) {
        let Some(table) = e.configured_endpoint(
            r.target,
            "transit_gateway_route_table_id",
            "aws_ec2_transit_gateway_route_table",
            &["id"],
        ) else {
            return false;
        };
        return e.same_endpoint(
            r.target,
            "transit_gateway_id",
            table,
            "transit_gateway_id",
            "aws_ec2_transit_gateway",
            "id",
        );
    }
    if r.attribute != "transit_gateway_attachment_id" {
        return true;
    }
    let Some(table) = e.configured_endpoint(
        r.target,
        "transit_gateway_route_table_id",
        "aws_ec2_transit_gateway_route_table",
        &["id"],
    ) else {
        return false;
    };
    let Some(gateway) = gateway(e, table) else {
        return false;
    };
    attachment_on(e, source, gateway)
        && (!matches!(
            kind,
            "aws_ec2_transit_gateway_route" | "aws_ec2_transit_gateway_prefix_list_reference"
        ) || e.value(r.target, "blackhole").and_then(Value::as_bool) == Some(false))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transit_routes.rs"]
mod tests;

pub(super) fn gateway(e: &Evidence<'_>, node: usize) -> Option<usize> {
    e.configured_endpoint(
        node,
        "transit_gateway_id",
        "aws_ec2_transit_gateway",
        &["id"],
    )
}
pub(super) fn attachment_on(e: &Evidence<'_>, source: usize, expected: usize) -> bool {
    gateway(e, source) == Some(expected)
}
