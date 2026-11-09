use crate::model::{Action, AttributeReference, AwsAccountScope, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/ipam.rs"]
mod tests;

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        "aws_ec2_subnet_cidr_reservation" => {
            Some(("subnet_id", "Reservation subnet", "CIDR reservations"))
        }
        "aws_vpc_ipam_scope" => Some(("ipam_id", "Scope IPAM", "IPAM scopes")),
        "aws_vpc_ipam_pool_cidr"
        | "aws_vpc_ipam_pool_cidr_allocation"
        | "aws_vpc_ipam_preview_next_cidr" => {
            Some(("ipam_pool_id", "IPAM pool owner", "Pool configuration"))
        }
        "aws_vpc_ipv4_cidr_block_association" | "aws_vpc_ipv6_cidr_block_association" => {
            Some(("vpc_id", "CIDR VPC owner", "CIDR associations"))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if let Some((field, label, _)) = owner(kind) {
        if path == field {
            return Some(label);
        }
    }
    match (kind, path) {
        ("aws_subnet" | "aws_vpc" | "aws_vpc_ipv4_cidr_block_association", "ipv4_ipam_pool_id") => {
            Some("IPv4 IPAM pool")
        }
        ("aws_subnet" | "aws_vpc" | "aws_vpc_ipv6_cidr_block_association", "ipv6_ipam_pool_id") => {
            Some("IPv6 IPAM pool")
        }
        ("aws_subnet", "outpost_arn") => Some("Subnet Outpost"),
        ("aws_vpc_ipam_organization_admin_account", "delegated_admin_account_id") => {
            Some("Delegated IPAM account")
        }
        ("aws_vpc_ipam_pool", "ipam_scope_id") => Some("Pool allocation scope"),
        ("aws_vpc_ipam_pool", "source_ipam_pool_id") => Some("Parent IPAM pool"),
        ("aws_vpc_ipam_pool", "source_resource.resource_id") => Some("Planning VPC"),
        ("aws_vpc_ipam_resource_discovery_association", "ipam_id") => Some("Associated IPAM"),
        ("aws_vpc_ipam_resource_discovery_association", "ipam_resource_discovery_id") => {
            Some("Associated discovery")
        }
        _ => None,
    }
}

fn region(node: &TerraformEntity) -> Option<String> {
    super::scope::scope(node)?.region.map(|r| r.0)
}

fn scalar<'a>(evidence: &[(&'a Value, &'a Value)], index: usize, path: &str) -> Option<&'a str> {
    let (value, mask) = evidence[index];
    super::streaming::values(value, mask, path, false)?
        .into_iter()
        .next()
}

fn unique<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut candidates = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = candidates.next()?;
    candidates.next().is_none().then_some(r)
}

fn endpoint(kind: &str, path: &str, source: &str, field: &str) -> bool {
    matches!(
        (kind, path, source, field),
        (
            "aws_ec2_subnet_cidr_reservation",
            "subnet_id",
            "aws_subnet",
            "id"
        ) | ("aws_vpc_ipam_scope", "ipam_id", "aws_vpc_ipam", "id")
            | (
                "aws_vpc_ipam_pool",
                "ipam_scope_id",
                "aws_vpc_ipam_scope",
                "id"
            )
            | (
                "aws_vpc_ipam_pool",
                "source_ipam_pool_id",
                "aws_vpc_ipam_pool",
                "id"
            )
            | (
                "aws_vpc_ipam_pool",
                "source_resource.resource_id",
                "aws_vpc",
                "id"
            )
            | (
                "aws_vpc_ipam_pool_cidr"
                    | "aws_vpc_ipam_pool_cidr_allocation"
                    | "aws_vpc_ipam_preview_next_cidr",
                "ipam_pool_id",
                "aws_vpc_ipam_pool",
                "id",
            )
            | (
                "aws_vpc_ipv4_cidr_block_association" | "aws_vpc_ipv6_cidr_block_association",
                "vpc_id",
                "aws_vpc",
                "id",
            )
            | (
                "aws_subnet" | "aws_vpc" | "aws_vpc_ipv4_cidr_block_association",
                "ipv4_ipam_pool_id",
                "aws_vpc_ipam_pool",
                "id",
            )
            | (
                "aws_subnet" | "aws_vpc" | "aws_vpc_ipv6_cidr_block_association",
                "ipv6_ipam_pool_id",
                "aws_vpc_ipam_pool",
                "id",
            )
            | ("aws_subnet", "outpost_arn", "aws_outposts_outpost", "arn")
            | (
                "aws_vpc_ipam_organization_admin_account",
                "delegated_admin_account_id",
                "aws_organizations_account",
                "id",
            )
            | (
                "aws_vpc_ipam_resource_discovery_association",
                "ipam_id",
                "aws_vpc_ipam",
                "id"
            )
            | (
                "aws_vpc_ipam_resource_discovery_association",
                "ipam_resource_discovery_id",
                "aws_vpc_ipam_resource_discovery",
                "id",
            )
    )
}

struct Proof {
    owner: Option<AwsAccountScope>,
}

fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<Proof> {
    let node = &nodes[r.target];
    let [source] = r.sources.as_slice() else {
        return None;
    };
    let target = &nodes[*source];
    let outpost = node.resource_type == "aws_subnet" && r.attribute == "outpost_arn";
    let current = |n: &TerraformEntity| {
        n.provider.is_aws() && n.action != Action::Delete && n.deposed_key.is_none()
    };
    if !current(node)
        || !current(target)
        || node.mode != EntityMode::Managed
        || target.mode
            != if outpost {
                EntityMode::Data
            } else {
                EntityMode::Managed
            }
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.source_attributes.len() != 1
        || unique(refs, r.target, &r.attribute).is_none()
        || r.target == *source
    {
        return None;
    }
    let field = r.source_attributes.first()?.as_str();
    if !endpoint(
        &node.resource_type,
        &r.attribute,
        &target.resource_type,
        field,
    ) {
        return None;
    }
    let actual = scalar(evidence, r.target, &r.attribute)?;
    if scalar(evidence, *source, field) != Some(actual) {
        return None;
    }
    let known = |p| scalar(evidence, r.target, p);
    let source_known = |p| scalar(evidence, *source, p);
    let mut owner = None;
    match (node.resource_type.as_str(), r.attribute.as_str()) {
        ("aws_ec2_subnet_cidr_reservation", _) => {
            if !matches!(known("reservation_type"), Some("explicit" | "prefix")) {
                return None;
            }
        }
        (_, "ipv4_ipam_pool_id" | "ipv6_ipam_pool_id") => {
            let family = if r.attribute == "ipv4_ipam_pool_id" {
                "ipv4"
            } else {
                "ipv6"
            };
            if source_known("address_family") != Some(family)
                || source_known("locale") != Some(region(node)?.as_str())
            {
                return None;
            }
            if family == "ipv6" && node.resource_type != "aws_subnet" {
                let (v, m) = evidence[r.target];
                if !super::vpc_gateways::absent(v, m, "ipv6_pool")
                    || !matches!(
                        &m["assign_generated_ipv6_cidr_block"],
                        Value::Null | Value::Bool(false)
                    )
                    || !matches!(
                        &v["assign_generated_ipv6_cidr_block"],
                        Value::Null | Value::Bool(false)
                    )
                {
                    return None;
                }
            }
        }
        ("aws_subnet", "outpost_arn") => {
            if target.has_region_override {
                return None;
            }
            let mut zone = false;
            for p in ["availability_zone", "availability_zone_id"] {
                if let Some(value) = known(p) {
                    if source_known(p) != Some(value) {
                        return None;
                    }
                    zone = true;
                }
            }
            if !zone {
                return None;
            }
            owner = Some(AwsAccountScope::new(source_known("owner_id")?)?);
        }
        ("aws_vpc_ipam_organization_admin_account", _) => {
            AwsAccountScope::new(actual)?;
        }
        ("aws_vpc_ipam_pool", "source_ipam_pool_id") => {
            if !matches!(known("address_family"), Some("ipv4" | "ipv6"))
                || known("address_family") != source_known("address_family")
                || known("ipam_scope_id").is_none()
                || known("ipam_scope_id") != source_known("ipam_scope_id")
            {
                return None;
            }
        }
        ("aws_vpc_ipam_pool", "source_resource.resource_id") => {
            if known("source_resource.resource_type") != Some("vpc") {
                return None;
            }
            let source_region = region(target)?;
            if known("source_resource.resource_region") != Some(source_region.as_str()) {
                return None;
            }
            owner = Some(AwsAccountScope::new(known(
                "source_resource.resource_owner",
            )?)?);
            let parent = unique(refs, r.target, "source_ipam_pool_id")?;
            proof(nodes, refs, parent, evidence)?;
            if scalar(evidence, parent.sources[0], "locale") != Some(source_region.as_str()) {
                return None;
            }
            let (v, m) = evidence[r.target];
            if !super::vpc_gateways::absent(v, m, "locale")
                && known("locale") != Some(source_region.as_str())
            {
                return None;
            }
        }
        ("aws_vpc_ipam_resource_discovery_association", "ipam_id") => {
            if known("ipam_region") != Some(region(target)?.as_str()) {
                return None;
            }
        }
        ("aws_vpc_ipam_resource_discovery_association", "ipam_resource_discovery_id") => {
            if region(node) != region(target) || region(node).is_none() {
                return None;
            }
            owner = Some(AwsAccountScope::new(known("owner_id")?)?);
        }
        _ => {}
    }
    Some(Proof { owner })
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let proofs: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)
                .and_then(|_| proof(nodes, refs, r, evidence))
        })
        .collect();
    let pairs: std::collections::BTreeSet<_> = refs
        .iter()
        .filter(|r| nodes[r.target].resource_type == "aws_vpc_ipam_resource_discovery_association")
        .filter(|r| {
            ["ipam_id", "ipam_resource_discovery_id"].iter().all(|p| {
                refs.iter().enumerate().any(|(i, other)| {
                    other.target == r.target && other.attribute == *p && proofs[i].is_some()
                })
            })
        })
        .map(|r| r.target)
        .collect();
    for (r, p) in refs.iter_mut().zip(proofs) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = p.is_some()
            && (nodes[r.target].resource_type != "aws_vpc_ipam_resource_discovery_association"
                || pairs.contains(&r.target));
        r.service_reference_owner_account = p.and_then(|p| p.owner);
    }
}

fn accounts(
    left: &crate::model::ProviderConfiguration,
    right: &crate::model::ProviderConfiguration,
) -> bool {
    !left.account_conflict
        && !right.account_conflict
        && match (&left.account, &right.account) {
            (Some(a), Some(b)) => a == b,
            (None, None) => left == right,
            _ => false,
        }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let node = &raw.nodes[r.target];
    let Some(left) = node.provider_configuration.as_ref() else {
        return false;
    };
    let [source] = r.sources.as_slice() else {
        return false;
    };
    let target = &raw.nodes[*source];
    let Some(right) = target.provider_configuration.as_ref() else {
        return false;
    };
    if node.resource_type == "aws_vpc_ipam_resource_discovery_association" {
        let Some(ipam) =
            unique(&raw.attributes, r.target, "ipam_id").filter(|r| r.service_reference_verified)
        else {
            return false;
        };
        let Some(discovery) = unique(&raw.attributes, r.target, "ipam_resource_discovery_id")
            .filter(|r| r.service_reference_verified)
        else {
            return false;
        };
        let Some(ipam_config) = raw.nodes[ipam.sources[0]].provider_configuration.as_ref() else {
            return false;
        };
        let Some(discovery_config) = raw.nodes[discovery.sources[0]]
            .provider_configuration
            .as_ref()
        else {
            return false;
        };
        return accounts(left, ipam_config)
            && left.account.is_some()
            && !discovery_config.account_conflict
            && discovery.service_reference_owner_account.is_some()
            && discovery.service_reference_owner_account == discovery_config.account;
    }
    if !accounts(left, right) {
        return false;
    }
    if node.resource_type == "aws_vpc_ipam_organization_admin_account" {
        return true;
    }
    if r.attribute == "source_resource.resource_id" {
        let Some(parent) = unique(&raw.attributes, r.target, "source_ipam_pool_id")
            .filter(|parent| parent.service_reference_verified)
        else {
            return false;
        };
        if !bindings(raw, parent) {
            return false;
        }
    }
    if matches!(
        r.attribute.as_str(),
        "source_resource.resource_id" | "outpost_arn"
    ) && (r.service_reference_owner_account.is_none()
        || r.service_reference_owner_account != right.account)
    {
        return false;
    }
    if matches!(
        r.attribute.as_str(),
        "ipv4_ipam_pool_id" | "ipv6_ipam_pool_id" | "source_resource.resource_id"
    ) {
        return node
            .entity
            .scope
            .as_ref()
            .and_then(|s| s.region.as_ref())
            .is_some()
            && target
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref())
                .is_some();
    }
    let target_region = if r.attribute == "outpost_arn" {
        right.region.as_ref()
    } else {
        target.entity.scope.as_ref().and_then(|s| s.region.as_ref())
    };
    node.entity
        .scope
        .as_ref()
        .and_then(|s| s.region.as_ref())
        .is_some()
        && node.entity.scope.as_ref().and_then(|s| s.region.as_ref()) == target_region
}

pub(super) fn association(raw: &Input, target: usize) -> Option<(usize, usize)> {
    let get = |path| {
        let r = unique(&raw.attributes, target, path)?;
        super::service_references::verified(raw, r).then_some(r.sources[0])
    };
    Some((get("ipam_id")?, get("ipam_resource_discovery_id")?))
}
