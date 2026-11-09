use crate::model::{
    AttributeReference, AwsAccountScope, DirectConnectProof, RegionScope, TerraformEntity,
};
use crate::provider::aws;
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../../tests/unit/semantic/direct_connect_hosted.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_dx_hosted_connection"
            | "aws_dx_connection_confirmation"
            | "aws_dx_hosted_private_virtual_interface"
            | "aws_dx_hosted_public_virtual_interface"
            | "aws_dx_hosted_transit_virtual_interface"
            | "aws_dx_hosted_private_virtual_interface_accepter"
            | "aws_dx_hosted_public_virtual_interface_accepter"
            | "aws_dx_hosted_transit_virtual_interface_accepter"
            | "aws_dx_gateway_association"
            | "aws_dx_gateway_association_proposal"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_dx_hosted_connection", "connection_id") => Some("Hosted connection LAG"),
        ("aws_dx_connection_confirmation", "connection_id") => Some("Confirmed hosted connection"),
        (
            "aws_dx_hosted_private_virtual_interface"
            | "aws_dx_hosted_public_virtual_interface"
            | "aws_dx_hosted_transit_virtual_interface",
            "connection_id",
        ) => Some("Allocated interface connection"),
        (
            "aws_dx_hosted_private_virtual_interface_accepter"
            | "aws_dx_hosted_public_virtual_interface_accepter"
            | "aws_dx_hosted_transit_virtual_interface_accepter",
            "virtual_interface_id",
        ) => Some("Accepted hosted interface"),
        (
            "aws_dx_hosted_private_virtual_interface_accepter"
            | "aws_dx_hosted_transit_virtual_interface_accepter"
            | "aws_dx_gateway_association"
            | "aws_dx_gateway_association_proposal",
            "dx_gateway_id",
        ) => Some("Direct Connect gateway"),
        ("aws_dx_hosted_private_virtual_interface_accepter", "vpn_gateway_id") => {
            Some("Virtual private gateway")
        }
        (
            "aws_dx_gateway_association" | "aws_dx_gateway_association_proposal",
            "associated_gateway_id",
        ) => Some("Associated regional gateway"),
        ("aws_dx_gateway_association", "proposal_id") => Some("Accepted gateway proposal"),
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    let valid = matches!(
        (kind, path, source),
        ("aws_dx_hosted_connection", "connection_id", "aws_dx_lag")
            | (
                "aws_dx_connection_confirmation",
                "connection_id",
                "aws_dx_hosted_connection"
            )
            | (
                "aws_dx_hosted_private_virtual_interface"
                    | "aws_dx_hosted_public_virtual_interface"
                    | "aws_dx_hosted_transit_virtual_interface",
                "connection_id",
                "aws_dx_connection" | "aws_dx_lag",
            )
            | (
                "aws_dx_hosted_private_virtual_interface_accepter",
                "virtual_interface_id",
                "aws_dx_hosted_private_virtual_interface",
            )
            | (
                "aws_dx_hosted_public_virtual_interface_accepter",
                "virtual_interface_id",
                "aws_dx_hosted_public_virtual_interface",
            )
            | (
                "aws_dx_hosted_transit_virtual_interface_accepter",
                "virtual_interface_id",
                "aws_dx_hosted_transit_virtual_interface",
            )
            | (_, "dx_gateway_id", "aws_dx_gateway")
            | (_, "vpn_gateway_id", "aws_vpn_gateway")
            | (
                _,
                "associated_gateway_id",
                "aws_vpn_gateway" | "aws_ec2_transit_gateway"
            )
            | (
                "aws_dx_gateway_association",
                "proposal_id",
                "aws_dx_gateway_association_proposal"
            )
    );
    if valid { &["id"] } else { &[] }
}
fn account(e: &[(&Value, &Value)], n: usize, path: &str) -> Option<AwsAccountScope> {
    AwsAccountScope::new(aws::service_evidence::known(e, n, path)?)
}
fn assign(p: &mut DirectConnectProof, n: usize, account: AwsAccountScope) -> Option<()> {
    if p.accounts.get(&n).is_some_and(|a| a != &account) {
        return None;
    }
    p.accounts.insert(n, account);
    p.participants.insert(n);
    Some(())
}
fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    n: usize,
    path: &str,
) -> Option<usize> {
    let r = aws::lambda_core::unique(refs, n, path)?;
    super::basic(nodes, refs, e, r).then_some(())?;
    (r.sources.len() == 1).then(|| r.sources[0])
}
fn proposal(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    n: usize,
    p: &mut DirectConnectProof,
) -> Option<(usize, usize)> {
    let dx = endpoint(nodes, refs, e, n, "dx_gateway_id")?;
    let regional = endpoint(nodes, refs, e, n, "associated_gateway_id")?;
    let dx_owner = account(e, n, "dx_gateway_owner_account_id")?;
    let regional_owner = account(e, n, "associated_gateway_owner_account_id")?;
    assign(p, dx, dx_owner)?;
    assign(p, regional, regional_owner.clone())?;
    assign(p, n, regional_owner)?;
    Some((dx, regional))
}
pub(super) fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<DirectConnectProof> {
    let t = r.target;
    if !resource(&nodes[t].resource_type) || !super::basic(nodes, refs, e, r) {
        return None;
    }
    let s = *r.sources.first()?;
    let mut p = DirectConnectProof::default();
    p.participants.extend([t, s]);
    match nodes[t].resource_type.as_str() {
        "aws_dx_hosted_connection" => {
            account(e, t, "owner_account_id")?;
            p.connection_region = Some(RegionScope(
                aws::service_evidence::known(e, t, "connection_region")?.into(),
            ));
            p.same_accounts.push((t, s));
        }
        "aws_dx_connection_confirmation" => {
            assign(&mut p, t, account(e, s, "owner_account_id")?)?;
            p.connection_region = Some(RegionScope(
                aws::service_evidence::known(e, s, "connection_region")?.into(),
            ));
        }
        "aws_dx_hosted_private_virtual_interface"
        | "aws_dx_hosted_public_virtual_interface"
        | "aws_dx_hosted_transit_virtual_interface" => {
            account(e, t, "owner_account_id")?;
            p.same_accounts.push((t, s));
        }
        "aws_dx_hosted_private_virtual_interface_accepter"
        | "aws_dx_hosted_public_virtual_interface_accepter"
        | "aws_dx_hosted_transit_virtual_interface_accepter" => {
            let allocator = endpoint(nodes, refs, e, t, "virtual_interface_id")?;
            p.participants.insert(allocator);
            assign(&mut p, t, account(e, allocator, "owner_account_id")?)?;
            if r.attribute != "virtual_interface_id" {
                p.same_accounts.push((t, s));
            }
        }
        "aws_dx_gateway_association_proposal" => {
            proposal(nodes, refs, e, t, &mut p)?;
        }
        "aws_dx_gateway_association" => {
            let dx = endpoint(nodes, refs, e, t, "dx_gateway_id")?;
            let dx_owner = account(e, t, "dx_gateway_owner_account_id")?;
            let regional_owner = account(e, t, "associated_gateway_owner_account_id")?;
            assign(&mut p, t, dx_owner.clone())?;
            assign(&mut p, dx, dx_owner.clone())?;
            if let Some(pr) =
                aws::lambda_core::unique(refs, t, "proposal_id").filter(|r| r.has_references)
            {
                if aws::lambda_core::unique(refs, t, "associated_gateway_id")
                    .is_some_and(|r| r.has_references)
                    || r.attribute == "associated_gateway_id"
                {
                    return None;
                }
                let source = endpoint(nodes, refs, e, t, "proposal_id")?;
                if pr.sources != [source] {
                    return None;
                }
                let (proposal_dx, regional) = proposal(nodes, refs, e, source, &mut p)?;
                if proposal_dx != dx {
                    return None;
                }
                assign(&mut p, regional, regional_owner)?;
            } else {
                let (v, m) = e[t];
                if !super::absent(v, m, "proposal_id") || dx_owner != regional_owner {
                    return None;
                }
                let regional = endpoint(nodes, refs, e, t, "associated_gateway_id")?;
                assign(&mut p, regional, regional_owner)?;
            }
        }
        _ => return None,
    }
    Some(p)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let Some(p) = &r.direct_connect_proof else {
        return false;
    };
    let account = |n: usize| {
        raw.nodes[n]
            .provider_configuration
            .as_ref()
            .filter(|c| !c.account_conflict)
            .and_then(|c| c.account.as_ref())
    };
    let region = p.connection_region.as_ref().or_else(|| {
        raw.nodes[r.target]
            .entity
            .scope
            .as_ref()
            .and_then(|s| s.region.as_ref())
    });
    region.is_some()
        && p.participants.iter().all(|&n| {
            let node = &raw.nodes[n];
            account(n).is_some()
                && p.accounts.get(&n).is_none_or(|a| account(n) == Some(a))
                && (node.resource_type == "aws_dx_gateway"
                    || (node.resource_type == "aws_dx_hosted_connection"
                        && p.connection_region.is_some())
                    || node.entity.scope.as_ref().and_then(|s| s.region.as_ref()) == region)
        })
        && p.same_accounts
            .iter()
            .all(|&(a, b)| account(a) == account(b))
}
