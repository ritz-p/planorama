use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_ec2_transit_gateway_peering_attachment" | "aws_ec2_transit_gateway_connect",
            "transit_gateway_id",
        ) => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_vpc_attachment", "transit_gateway_id") => Rule::new(
            "Transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ec2_transit_gateway_vpc_attachment", "vpc_id") => {
            Rule::new("Attached VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_ec2_transit_gateway_vpc_attachment", "subnet_ids") => {
            Rule::new("Attachment subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_ec2_transit_gateway_peering_attachment", "peer_transit_gateway_id") => Rule::new(
            "Peer transit gateway",
            &[("aws_ec2_transit_gateway", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_ec2_transit_gateway_peering_attachment_accepter",
            "transit_gateway_attachment_id",
        ) => Rule::new(
            "Peering request",
            &[("aws_ec2_transit_gateway_peering_attachment", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ec2_transit_gateway_vpc_attachment_accepter", "transit_gateway_attachment_id") => {
            Rule::new(
                "VPC attachment request",
                &[("aws_ec2_transit_gateway_vpc_attachment", &["id"])],
                false,
                &[],
            )
            .with_explicit_destination()
        }
        ("aws_ec2_transit_gateway_connect", "transport_attachment_id") => Rule::new(
            "Connect transport",
            &[("aws_ec2_transit_gateway_vpc_attachment", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_transit_gateway_connect_peer", "transit_gateway_attachment_id") => Rule::new(
            "Connect attachment",
            &[("aws_ec2_transit_gateway_connect", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_transit_gateway_vpc_attachment"
            | "aws_ec2_transit_gateway_peering_attachment"
            | "aws_ec2_transit_gateway_connect"
            | "aws_ec2_transit_gateway_peering_attachment_accepter"
            | "aws_ec2_transit_gateway_vpc_attachment_accepter"
            | "aws_ec2_transit_gateway_connect_peer"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule) && scoped(raw, r)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    for r in refs.iter_mut().filter(|r| r.service_reference_verified) {
        let account = match (nodes[r.target].resource_type.as_str(), r.attribute.as_str()) {
            ("aws_ec2_transit_gateway_peering_attachment", "peer_transit_gateway_id") => {
                super::service_evidence::known(values, r.target, "peer_account_id")
            }
            ("aws_ec2_transit_gateway_vpc_attachment", "transit_gateway_id") => r
                .sources
                .first()
                .and_then(|&s| super::service_evidence::known(values, s, "owner_id")),
            _ => continue,
        };
        r.service_reference_owner_account = account.and_then(crate::model::AwsAccountScope::new);
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    let kind = e.nodes[r.target].resource_type.as_str();
    match (kind, r.attribute.as_str()) {
        ("aws_ec2_transit_gateway_vpc_attachment", "vpc_id" | "subnet_ids") => e
            .configured_endpoint(r.target, "vpc_id", "aws_vpc", &["id"])
            .zip(configured_links::network_context(
                e,
                r.target,
                &[("subnet_ids", "aws_subnet")],
            ))
            .is_some_and(|(v, s)| v == s),
        ("aws_ec2_transit_gateway_connect", "transit_gateway_id" | "transport_attachment_id") => {
            let Some(transport) = e.configured_endpoint(
                r.target,
                "transport_attachment_id",
                "aws_ec2_transit_gateway_vpc_attachment",
                &["id"],
            ) else {
                return false;
            };
            e.same_endpoint(
                r.target,
                "transit_gateway_id",
                transport,
                "transit_gateway_id",
                "aws_ec2_transit_gateway",
                "id",
            )
        }
        ("aws_ec2_transit_gateway_peering_attachment", "peer_transit_gateway_id") => {
            e.source_region(r.target, "peer_region", source)
                && (e.absent(r.target, "peer_account_id")
                    || e.text(r.target, "peer_account_id")
                        .and_then(crate::model::AwsAccountScope::new)
                        .is_some())
        }
        ("aws_ec2_transit_gateway_peering_attachment_accepter", _) => {
            e.source_region(source, "peer_region", r.target)
        }
        ("aws_ec2_transit_gateway_vpc_attachment_accepter", _) => {
            super::scope::scope(&e.nodes[source])
                .and_then(|s| s.region)
                .zip(super::scope::scope(&e.nodes[r.target]).and_then(|s| s.region))
                .is_some_and(|(a, b)| a == b)
        }
        ("aws_ec2_transit_gateway_vpc_attachment", "transit_gateway_id") => {
            super::scope::scope(&e.nodes[source])
                .and_then(|s| s.region)
                .zip(super::scope::scope(&e.nodes[r.target]).and_then(|s| s.region))
                .is_some_and(|(a, b)| a == b)
                && (e.absent(source, "owner_id")
                    || e.text(source, "owner_id")
                        .and_then(crate::model::AwsAccountScope::new)
                        .is_some())
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transit_attachments.rs"]
mod tests;

fn scoped(raw: &Input, r: &AttributeReference) -> bool {
    let kind = raw.nodes[r.target].resource_type.as_str();
    if !matches!(
        (kind, r.attribute.as_str()),
        (
            "aws_ec2_transit_gateway_peering_attachment",
            "peer_transit_gateway_id"
        ) | (
            "aws_ec2_transit_gateway_vpc_attachment",
            "transit_gateway_id"
        ) | (
            "aws_ec2_transit_gateway_peering_attachment_accepter"
                | "aws_ec2_transit_gateway_vpc_attachment_accepter",
            "transit_gateway_attachment_id"
        )
    ) {
        return true;
    }
    let [source] = r.sources.as_slice() else {
        return false;
    };
    if kind.ends_with("_accepter") {
        let selector = if kind == "aws_ec2_transit_gateway_peering_attachment_accepter" {
            "peer_transit_gateway_id"
        } else {
            "transit_gateway_id"
        };
        let Some(gateway) =
            super::lambda_core::unique(&raw.attributes, *source, selector).filter(|g| {
                g.service_reference_verified
                    && scoped(raw, g)
                    && configured_links::bindings(raw, g, rule)
            })
        else {
            return false;
        };
        let [gateway] = gateway.sources.as_slice() else {
            return false;
        };
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[*gateway];
        return super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            false,
        );
    }
    if let Some(owner) = &r.service_reference_owner_account {
        return raw.nodes[r.target]
            .provider_configuration
            .as_ref()
            .is_some_and(|p| p.account.is_some() && !p.account_conflict)
            && raw.nodes[*source]
                .provider_configuration
                .as_ref()
                .is_some_and(|p| !p.account_conflict && p.account.as_ref() == Some(owner));
    }
    configured_links::destination_in_same_account(raw, r)
}
