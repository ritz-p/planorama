use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_ec2_carrier_gateway" | "aws_ec2_local_gateway_route_table_vpc_association",
            "vpc_id",
        ) => Rule::new("Gateway VPC", &[("aws_vpc", &["id"])], false, &[]),
        ("aws_ec2_local_gateway_route_table", "local_gateway_id") => Rule::new(
            "Local gateway",
            &[("aws_ec2_local_gateway", &["id"])],
            false,
            &[],
        )
        .data_source(),
        (
            "aws_ec2_local_gateway_route"
            | "aws_ec2_local_gateway_route_table_virtual_interface_group_association"
            | "aws_ec2_local_gateway_route_table_vpc_association",
            "local_gateway_route_table_id",
        ) => Rule::new(
            "Local route table",
            &[(
                "aws_ec2_local_gateway_route_table",
                &["local_gateway_route_table_id"],
            )],
            false,
            &[],
        ),
        (
            "aws_ec2_local_gateway_route"
            | "aws_ec2_local_gateway_route_table_virtual_interface_group_association",
            "local_gateway_virtual_interface_group_id",
        ) => Rule::new(
            "Local virtual interface group",
            &[("aws_ec2_local_gateway_virtual_interface_group", &["id"])],
            false,
            &[],
        )
        .data_source(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_carrier_gateway"
            | "aws_ec2_local_gateway_route_table_vpc_association"
            | "aws_ec2_local_gateway_route_table"
            | "aws_ec2_local_gateway_route"
            | "aws_ec2_local_gateway_route_table_virtual_interface_group_association"
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
fn qualifies(_e: &Evidence<'_>, _r: &AttributeReference, _source: usize, _path: &str) -> bool {
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/local_gateways.rs"]
mod tests;
