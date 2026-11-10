use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/networkmanager.rs"]
mod tests;
const GLOBAL: &str = "aws_networkmanager_global_network";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        GLOBAL
            | "aws_networkmanager_core_network"
            | "aws_networkmanager_site"
            | "aws_networkmanager_device"
            | "aws_networkmanager_link"
            | "aws_networkmanager_connection"
            | "aws_networkmanager_link_association"
            | "aws_networkmanager_customer_gateway_association"
            | "aws_networkmanager_transit_gateway_connect_peer_association"
            | "aws_networkmanager_transit_gateway_registration"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if resource(kind) && kind != GLOBAL && path == "global_network_id" {
        return Some("Global network");
    }
    match (kind, path) {
        ("aws_networkmanager_device" | "aws_networkmanager_link", "site_id") => {
            Some("Network Manager site")
        }
        ("aws_networkmanager_device", "aws_location.subnet_arn") => {
            Some("Device AWS subnet context")
        }
        (
            "aws_networkmanager_connection"
            | "aws_networkmanager_link_association"
            | "aws_networkmanager_customer_gateway_association"
            | "aws_networkmanager_transit_gateway_connect_peer_association",
            "device_id",
        ) => Some("Network Manager device"),
        ("aws_networkmanager_connection", "connected_device_id") => Some("Connected device"),
        (
            "aws_networkmanager_connection"
            | "aws_networkmanager_link_association"
            | "aws_networkmanager_customer_gateway_association"
            | "aws_networkmanager_transit_gateway_connect_peer_association",
            "link_id",
        ) => Some("Network Manager link"),
        ("aws_networkmanager_connection", "connected_link_id") => Some("Connected link"),
        ("aws_networkmanager_customer_gateway_association", "customer_gateway_arn") => {
            Some("Associated customer gateway")
        }
        (
            "aws_networkmanager_transit_gateway_connect_peer_association",
            "transit_gateway_connect_peer_arn",
        ) => Some("Associated transit gateway peer"),
        ("aws_networkmanager_transit_gateway_registration", "transit_gateway_arn") => {
            Some("Registered transit gateway")
        }
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("global_network_id", GLOBAL)
        | ("site_id", "aws_networkmanager_site")
        | ("device_id" | "connected_device_id", "aws_networkmanager_device")
        | ("link_id" | "connected_link_id", "aws_networkmanager_link") => &["id"],
        ("aws_location.subnet_arn", "aws_subnet")
        | ("customer_gateway_arn", "aws_customer_gateway")
        | ("transit_gateway_connect_peer_arn", "aws_ec2_transit_gateway_connect_peer")
        | ("transit_gateway_arn", "aws_ec2_transit_gateway") => &["arn"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == "aws_networkmanager_device" && path == "aws_location.0.subnet_arn"
}
fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
        super::nested_references::value(e[r.target].0, e[r.target].1, path)
            .is_some_and(Value::is_string)
    })
    .is_some()
}
fn global(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, t, "global_network_id")?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    basic(nodes, refs, e, r).then_some(*s)
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let mut valid = basic(nodes, refs, e, r);
            let mut context = Vec::new();
            if valid && r.attribute != "global_network_id" {
                if let Some(parent) = global(nodes, refs, e, r.target) {
                    context.push(parent);
                    valid = r.sources.iter().all(|&s| {
                        !resource(&nodes[s].resource_type)
                            || global(nodes, refs, e, s) == Some(parent)
                    });
                } else {
                    valid = false;
                }
            }
            if valid && r.attribute == "aws_location.subnet_arn" {
                valid = e[r.target].0["aws_location"]
                    .as_array()
                    .is_some_and(|a| a.len() == 1)
                    && super::nested_references::value(
                        e[r.target].0,
                        e[r.target].1,
                        "aws_location.0.zone",
                    )
                    .is_some_and(|v| v.is_null() || v.as_str() == Some(""));
            }
            Some((valid, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
            if valid {
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
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let b = &raw.nodes[s];
            super::iam_policies::same_account(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
            ) && (resource(&b.resource_type)
                || b.entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref())
                    .is_some())
        })
}
