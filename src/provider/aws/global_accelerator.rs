use crate::model::{AttributeReference, ProviderConfiguration, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/global_accelerator.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_globalaccelerator_accelerator"
            | "aws_globalaccelerator_listener"
            | "aws_globalaccelerator_endpoint_group"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_globalaccelerator_accelerator", "attributes.flow_logs_s3_bucket") => {
            Some("Flow log bucket")
        }
        ("aws_globalaccelerator_listener", "accelerator_arn") => Some("Listener accelerator"),
        ("aws_globalaccelerator_endpoint_group", "listener_arn") => Some("Group listener"),
        ("aws_globalaccelerator_endpoint_group", "endpoint_configuration.endpoint_id") => {
            Some("Configured endpoints")
        }
        ("aws_globalaccelerator_endpoint_group", "endpoint_configuration.attachment_arn") => {
            Some("Endpoint cross-account attachment")
        }
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (
            "aws_globalaccelerator_accelerator",
            "attributes.flow_logs_s3_bucket",
            "aws_s3_bucket",
        ) => &["bucket"],
        (
            "aws_globalaccelerator_listener",
            "accelerator_arn",
            "aws_globalaccelerator_accelerator",
        )
        | (
            "aws_globalaccelerator_endpoint_group",
            "listener_arn",
            "aws_globalaccelerator_listener",
        )
        | (
            "aws_globalaccelerator_endpoint_group",
            "endpoint_configuration.endpoint_id",
            "aws_lb",
        )
        | (
            "aws_globalaccelerator_endpoint_group",
            "endpoint_configuration.attachment_arn",
            "aws_globalaccelerator_cross_account_attachment",
        ) => &["arn"],
        (
            "aws_globalaccelerator_endpoint_group",
            "endpoint_configuration.endpoint_id",
            "aws_eip",
        ) => &["allocation_id"],
        _ => &[],
    }
}
pub(super) fn valid(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
) -> bool {
    if !resource(&nodes[r.target].resource_type) {
        return true;
    }
    let (value, mask) = evidence[r.target];
    match r.attribute.as_str() {
        "attributes.flow_logs_s3_bucket" => {
            value["attributes"].as_array().is_some_and(|a| a.len() == 1)
                && value["attributes"][0]["flow_logs_enabled"] == true
                && matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
                && matches!(
                    &mask["attributes"],
                    Value::Null | Value::Bool(false) | Value::Array(_)
                )
                && matches!(
                    &mask["attributes"][0],
                    Value::Null | Value::Bool(false) | Value::Object(_)
                )
                && matches!(
                    &mask["attributes"][0]["flow_logs_enabled"],
                    Value::Null | Value::Bool(false)
                )
        }
        "endpoint_configuration.endpoint_id" => {
            let endpoint = &nodes[source];
            let region = super::scope::scope(endpoint).and_then(|s| s.region);
            region.is_some_and(|region| {
                super::service_evidence::known(evidence, r.target, "endpoint_group_region")
                    == Some(region.0.as_str())
            }) && (super::aliases::entity_type(endpoint) != "aws_lb"
                || matches!(
                    super::service_evidence::known(evidence, source, "load_balancer_type"),
                    Some("application" | "network")
                ))
        }
        _ => true,
    }
}
pub(super) fn compatible(
    left: Option<&ProviderConfiguration>,
    right: Option<&ProviderConfiguration>,
) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    !left.account_conflict
        && !right.account_conflict
        && match (&left.account, &right.account) {
            (Some(a), Some(b)) => a == b,
            (None, None) => left == right,
            _ => false,
        }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    !resource(&raw.nodes[r.target].resource_type)
        || r.sources.iter().all(|&s| {
            compatible(
                raw.nodes[r.target].provider_configuration.as_ref(),
                raw.nodes[s].provider_configuration.as_ref(),
            )
        })
}

pub(super) fn present(graph: &mut crate::model::Graph) {
    let groups: Vec<_> = graph
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            if node.resource_type != "aws_globalaccelerator_endpoint_group" {
                return None;
            }
            let owner = node.member_owner.as_ref()?;
            graph
                .nodes
                .iter()
                .any(|n| {
                    n.entity.provenance.contains(&owner.target)
                        && n.resource_type == "aws_globalaccelerator_listener"
                        && n.member_owner.is_some()
                        && n.placement == crate::model::Placement::Spatial
                })
                .then_some(index)
        })
        .collect();
    for index in groups {
        graph.nodes[index].placement = crate::model::Placement::Spatial;
    }
}
