use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

mod hosted;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/direct_connect.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    hosted::resource(kind)
        || matches!(
            kind,
            "aws_dx_connection"
                | "aws_dx_lag"
                | "aws_dx_gateway"
                | "aws_dx_connection_association"
                | "aws_dx_private_virtual_interface"
                | "aws_dx_public_virtual_interface"
                | "aws_dx_transit_virtual_interface"
                | "aws_dx_bgp_peer"
                | "aws_dx_macsec_key_association"
        )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if hosted::resource(kind) {
        return hosted::label(kind, path);
    }
    match (kind, path) {
        (
            "aws_dx_lag" | "aws_dx_connection_association" | "aws_dx_macsec_key_association",
            "connection_id",
        ) => Some("Dedicated connection"),
        (
            "aws_dx_private_virtual_interface"
            | "aws_dx_public_virtual_interface"
            | "aws_dx_transit_virtual_interface",
            "connection_id",
        ) => Some("Virtual interface connection"),
        ("aws_dx_connection_association", "lag_id") => Some("Connection LAG"),
        (
            "aws_dx_private_virtual_interface" | "aws_dx_transit_virtual_interface",
            "dx_gateway_id",
        ) => Some("Direct Connect gateway"),
        ("aws_dx_private_virtual_interface", "vpn_gateway_id") => Some("Virtual private gateway"),
        ("aws_dx_bgp_peer", "virtual_interface_id") => Some("Peer virtual interface"),
        ("aws_dx_macsec_key_association", "secret_arn") => Some("Configured MACsec secret"),
        _ => None,
    }
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    if hosted::resource(kind) {
        return hosted::fields(kind, path, source);
    }
    match (kind, path, source) {
        (_, "connection_id", "aws_dx_connection")
        | (
            "aws_dx_private_virtual_interface"
            | "aws_dx_public_virtual_interface"
            | "aws_dx_transit_virtual_interface",
            "connection_id",
            "aws_dx_lag",
        )
        | (_, "lag_id", "aws_dx_lag")
        | (_, "dx_gateway_id", "aws_dx_gateway")
        | (_, "vpn_gateway_id", "aws_vpn_gateway")
        | (
            _,
            "virtual_interface_id",
            "aws_dx_private_virtual_interface"
            | "aws_dx_public_virtual_interface"
            | "aws_dx_transit_virtual_interface",
        ) => &["id"],
        (_, "secret_arn", "aws_secretsmanager_secret") => &["arn"],
        _ => &[],
    }
}

fn absent(v: &Value, m: &Value, path: &str) -> bool {
    super::nested_references::value(v, m, path)
        .is_some_and(|v| v.is_null() || v.as_str() == Some(""))
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some((
                basic(nodes, refs, evidence, r),
                hosted::proof(nodes, refs, evidence, r),
            ))
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some((valid, proof)) = valid {
            r.service_reference_verified =
                valid && (!hosted::resource(&nodes[r.target].resource_type) || proof.is_some());
            r.direct_connect_proof = proof;
        }
    }
}

fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    label(&nodes[r.target].resource_type, &r.attribute).is_some()
        && super::nested_references::verify(nodes, refs, evidence, r, fields, |_, path| {
            let (v, m) = evidence[r.target];
            if !super::nested_references::value(v, m, path).is_some_and(Value::is_string) {
                return false;
            }
            if matches!(
                nodes[r.target].resource_type.as_str(),
                "aws_dx_private_virtual_interface"
                    | "aws_dx_hosted_private_virtual_interface_accepter"
            ) {
                match r.attribute.as_str() {
                    "dx_gateway_id" => absent(v, m, "vpn_gateway_id"),
                    "vpn_gateway_id" => absent(v, m, "dx_gateway_id"),
                    _ => true,
                }
            } else {
                true
            }
        })
        .is_some()
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    if hosted::resource(&raw.nodes[r.target].resource_type) {
        return hosted::bindings(raw, r);
    }
    r.sources.iter().all(|&s| {
        let a = &raw.nodes[r.target];
        let b = &raw.nodes[s];
        let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        super::iam_policies::same_account(
            a.provider_configuration.as_ref(),
            b.provider_configuration.as_ref(),
        ) && ar.is_some()
            && (b.resource_type == "aws_dx_gateway" || ar == br)
    })
}
