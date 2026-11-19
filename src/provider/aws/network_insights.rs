use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ec2_network_insights_path", "source") => Rule::new(
            "Analysis source",
            &[
                ("aws_instance", &["id", "arn"]),
                ("aws_internet_gateway", &["id", "arn"]),
                ("aws_network_interface", &["id", "arn"]),
                ("aws_ec2_transit_gateway", &["id", "arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["id", "arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["id", "arn"]),
                ("aws_vpn_gateway", &["id", "arn"]),
                ("aws_vpc_endpoint_service", &["id", "arn"]),
                ("aws_vpc_endpoint", &["id", "arn"]),
                ("aws_vpc_peering_connection", &["id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ec2_network_insights_path", "destination") => Rule::new(
            "Analysis destination",
            &[
                ("aws_instance", &["id", "arn"]),
                ("aws_internet_gateway", &["id", "arn"]),
                ("aws_network_interface", &["id", "arn"]),
                ("aws_ec2_transit_gateway", &["id", "arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["id", "arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["id", "arn"]),
                ("aws_vpn_gateway", &["id", "arn"]),
                ("aws_vpc_endpoint_service", &["id", "arn"]),
                ("aws_vpc_endpoint", &["id", "arn"]),
                ("aws_vpc_peering_connection", &["id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ec2_network_insights_analysis", "network_insights_path_id") => Rule::new(
            "Network insights path",
            &[("aws_ec2_network_insights_path", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_network_insights_analysis", "filter_in_arns") => Rule::new(
            "Configured intermediate resources",
            &[
                ("aws_lb", &["arn"]),
                ("aws_elb", &["arn"]),
                ("aws_networkfirewall_firewall", &["arn"]),
                ("aws_ec2_transit_gateway", &["arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["arn"]),
            ],
            true,
            &[],
        ),
        (
            "aws_ec2_network_insights_access_scope",
            "match_paths.source.resource_statement.resources",
        ) => Rule::new(
            "Match source resources",
            &[
                ("aws_resourcegroups_group", &["arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["arn"]),
                ("aws_vpc_endpoint_service", &["arn"]),
                ("aws_vpc_endpoint", &["arn"]),
                ("aws_instance", &["arn"]),
                ("aws_internet_gateway", &["arn"]),
                ("aws_network_interface", &["arn"]),
                ("aws_security_group", &["arn"]),
                ("aws_subnet", &["arn"]),
                ("aws_vpc", &["arn"]),
                ("aws_vpn_gateway", &["arn"]),
            ],
            true,
            &[
                "match_paths",
                "match_paths.source",
                "match_paths.source.resource_statement",
            ],
        ),
        (
            "aws_ec2_network_insights_access_scope",
            "match_paths.destination.resource_statement.resources",
        ) => Rule::new(
            "Match destination resources",
            &[
                ("aws_resourcegroups_group", &["arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["arn"]),
                ("aws_vpc_endpoint_service", &["arn"]),
                ("aws_vpc_endpoint", &["arn"]),
                ("aws_instance", &["arn"]),
                ("aws_internet_gateway", &["arn"]),
                ("aws_network_interface", &["arn"]),
                ("aws_security_group", &["arn"]),
                ("aws_subnet", &["arn"]),
                ("aws_vpc", &["arn"]),
                ("aws_vpn_gateway", &["arn"]),
            ],
            true,
            &[
                "match_paths",
                "match_paths.destination",
                "match_paths.destination.resource_statement",
            ],
        ),
        (
            "aws_ec2_network_insights_access_scope",
            "exclude_paths.through_resources.resource_statement.resources",
        ) => Rule::new(
            "Excluded intermediate resources",
            &[
                ("aws_resourcegroups_group", &["arn"]),
                ("aws_ec2_transit_gateway_vpc_attachment", &["arn"]),
                ("aws_ec2_transit_gateway_peering_attachment", &["arn"]),
                ("aws_vpc_endpoint_service", &["arn"]),
                ("aws_vpc_endpoint", &["arn"]),
                ("aws_lb", &["arn"]),
                ("aws_elb", &["arn"]),
                ("aws_networkfirewall_firewall", &["arn"]),
            ],
            true,
            &[
                "exclude_paths",
                "exclude_paths.through_resources",
                "exclude_paths.through_resources.resource_statement",
            ],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_network_insights_path"
            | "aws_ec2_network_insights_analysis"
            | "aws_ec2_network_insights_access_scope"
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
    match r.attribute.as_str() {
        "source" | "destination" => path_scope(e, r, source),
        path_name if path_name.ends_with(".resources") => path
            .strip_suffix(".resources")
            .and_then(|prefix| e.value(r.target, &format!("{prefix}.resource_types")))
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty)),
        _ => true,
    }
}
fn path_scope(e: &Evidence<'_>, r: &AttributeReference, source: usize) -> bool {
    let consumer = &e.nodes[r.target];
    let endpoint = &e.nodes[source];
    let left_scope = super::scope::scope(consumer).and_then(|s| s.region);
    let right_scope = super::scope::scope(endpoint).and_then(|s| s.region);
    let left = consumer.provider_configuration.as_ref();
    let right = endpoint.provider_configuration.as_ref();
    if super::eventbridge::bindings(
        left,
        right,
        left_scope.as_ref(),
        right_scope.as_ref(),
        false,
    ) {
        return true;
    }
    let Some(region) = left_scope.filter(|r| Some(r) == right_scope.as_ref()) else {
        return false;
    };
    let Some(arn) = e.text(source, "arn") else {
        return false;
    };
    if !r
        .source_attributes_by_instance
        .get(&source)
        .is_some_and(|outputs| outputs.len() == 1 && outputs.contains("arn"))
    {
        return false;
    }
    let mut parts = arn.splitn(6, ':');
    let [
        Some("arn"),
        Some("aws"),
        Some("ec2"),
        Some(arn_region),
        Some(account),
        Some(resource),
    ] = std::array::from_fn(|_| parts.next())
    else {
        return false;
    };
    arn_region == region.0
        && !resource.is_empty()
        && crate::model::AwsAccountScope::new(account).is_some()
        && right
            .and_then(|provider| provider.account.as_ref())
            .is_none_or(|scope| scope.as_str() == account)
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/network_insights.rs"]
mod tests;
