use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_ec2_transit_gateway_policy_table" | "aws_ec2_transit_gateway_metering_policy",
            "transit_gateway_id",
        ) => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_metering_policy", "middlebox_attachment_ids") => Rule::new(
            "Middlebox attachments",
            &[
                ("aws_ec2_transit_gateway_vpc_attachment", &["id"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["id"]),
                ("aws_ec2_transit_gateway_connect", &["id"]),
                ("aws_vpn_connection", &["transit_gateway_attachment_id"]),
                ("aws_vpn_concentrator", &["transit_gateway_attachment_id"]),
            ],
            true,
            &[],
        ),
        ("aws_ec2_transit_gateway_metering_policy_entry", "transit_gateway_metering_policy_id") => {
            Rule::new(
                "Metering policy",
                &[(
                    "aws_ec2_transit_gateway_metering_policy",
                    &["transit_gateway_metering_policy_id"],
                )],
                false,
                &[],
            )
        }
        (
            "aws_ec2_transit_gateway_metering_policy_entry",
            "source_transit_gateway_attachment_id",
        ) => Rule::new(
            "Metered source attachment",
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
        (
            "aws_ec2_transit_gateway_metering_policy_entry",
            "destination_transit_gateway_attachment_id",
        ) => Rule::new(
            "Metered destination attachment",
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
        (
            "aws_ec2_transit_gateway_policy_table_association"
            | "aws_ec2_transit_gateway_policy_table_entry",
            "transit_gateway_policy_table_id",
        ) => Rule::new(
            "Transit policy table",
            &[("aws_ec2_transit_gateway_policy_table", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_policy_table_association", "transit_gateway_attachment_id") => {
            Rule::new(
                "Policy attachment",
                &[
                    ("aws_ec2_transit_gateway_vpc_attachment", &["id"]),
                    ("aws_ec2_transit_gateway_peering_attachment", &["id"]),
                    ("aws_ec2_transit_gateway_connect", &["id"]),
                    ("aws_vpn_connection", &["transit_gateway_attachment_id"]),
                    ("aws_vpn_concentrator", &["transit_gateway_attachment_id"]),
                ],
                false,
                &[],
            )
        }
        ("aws_ec2_transit_gateway_policy_table_entry", "target_route_table_id") => Rule::new(
            "Target transit route table",
            &[("aws_ec2_transit_gateway_route_table", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_transit_gateway_policy_table"
            | "aws_ec2_transit_gateway_metering_policy"
            | "aws_ec2_transit_gateway_metering_policy_entry"
            | "aws_ec2_transit_gateway_policy_table_association"
            | "aws_ec2_transit_gateway_policy_table_entry"
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
    use super::transit_routes::{attachment_on, gateway};
    let field = r.attribute.as_str();
    if field == "middlebox_attachment_ids" {
        return gateway(e, r.target).is_some_and(|g| attachment_on(e, source, g));
    }
    let (selector, kind, out) = match field {
        "source_transit_gateway_attachment_id" | "destination_transit_gateway_attachment_id" => (
            "transit_gateway_metering_policy_id",
            "aws_ec2_transit_gateway_metering_policy",
            "transit_gateway_metering_policy_id",
        ),
        "transit_gateway_attachment_id" | "target_route_table_id" => (
            "transit_gateway_policy_table_id",
            "aws_ec2_transit_gateway_policy_table",
            "id",
        ),
        _ => return true,
    };
    let Some(g) = e
        .configured_endpoint(r.target, selector, kind, &[out])
        .and_then(|policy| gateway(e, policy))
    else {
        return false;
    };
    if field == "target_route_table_id" {
        return gateway(e, source) == Some(g);
    }
    if !attachment_on(e, source, g) {
        return false;
    }
    if field == "transit_gateway_attachment_id" {
        return true;
    }
    let selector = field.trim_end_matches("_id").to_owned() + "_type";
    e.absent(r.target, &selector)
        || matches!(
            (
                e.text(r.target, &selector),
                e.nodes[source].resource_type.as_str()
            ),
            (Some("vpc"), "aws_ec2_transit_gateway_vpc_attachment")
                | (Some("connect"), "aws_ec2_transit_gateway_connect")
                | (
                    Some("peering" | "tgw-peering"),
                    "aws_ec2_transit_gateway_peering_attachment"
                )
                | (Some("vpn"), "aws_vpn_connection")
                | (Some("vpn-concentrator"), "aws_vpn_concentrator")
        )
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transit_policies.rs"]
mod tests;
