use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/vpc_gateways.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static str, &'static str, bool)> {
    Some(match (kind, field) {
        (
            "aws_egress_only_internet_gateway"
            | "aws_internet_gateway"
            | "aws_internet_gateway_attachment"
            | "aws_nat_gateway",
            "vpc_id",
        ) => ("aws_vpc", "id", false),
        ("aws_internet_gateway_attachment", "internet_gateway_id") => {
            ("aws_internet_gateway", "id", false)
        }
        ("aws_eip", "instance") | ("aws_eip_association", "instance_id") => {
            ("aws_instance", "id", false)
        }
        ("aws_eip", "network_interface") | ("aws_eip_association", "network_interface_id") => {
            ("aws_network_interface", "id", false)
        }
        ("aws_eip", "ipam_pool_id") => ("aws_vpc_ipam_pool", "id", false),
        (
            "aws_eip_association"
            | "aws_eip_domain_name"
            | "aws_nat_gateway_eip_association"
            | "aws_nat_gateway",
            "allocation_id",
        ) => ("aws_eip", "allocation_id", false),
        (
            "aws_nat_gateway",
            "secondary_allocation_ids" | "availability_zone_address.allocation_ids",
        ) => ("aws_eip", "allocation_id", true),
        ("aws_nat_gateway_eip_association", "nat_gateway_id") => ("aws_nat_gateway", "id", false),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "vpc_id" => "Gateway VPC",
        "ipam_pool_id" => "Allocation pool",
        "allocation_id"
        | "secondary_allocation_ids"
        | "availability_zone_address.allocation_ids" => "Elastic IP allocation",
        _ => "Gateway association",
    })
}

pub(super) fn absent(value: &Value, unknown: &Value, field: &str) -> bool {
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&unknown[field], Value::Null | Value::Bool(false))
        && (value[field].is_null()
            || value[field].as_str() == Some("")
            || value[field].as_array().is_some_and(Vec::is_empty))
}

fn regional_values<'a>(value: &'a Value, unknown: &Value) -> Option<BTreeSet<&'a str>> {
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(
            &unknown["availability_zone_address"],
            Value::Null | Value::Bool(false)
        )
    {
        return None;
    }
    let entries = value["availability_zone_address"].as_array()?;
    if entries.is_empty() {
        return None;
    }
    let mut result = BTreeSet::new();
    let mut zones = BTreeSet::new();
    for entry in entries {
        let zone = entry["availability_zone_id"]
            .as_str()
            .filter(|s| !s.is_empty())?;
        if !zones.insert(zone) {
            return None;
        }
        let ids = super::streaming::values(entry, &Value::Null, "allocation_ids", true)?;
        for id in ids {
            if !result.insert(id) {
                return None;
            }
        }
    }
    Some(result)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs.iter_mut() {
        let node = &nodes[r.target];
        let (value, unknown) = evidence[r.target];
        let known = |key| super::service_evidence::known(evidence, r.target, key);
        if node.resource_type == "aws_nat_gateway" && r.attribute == "subnet_id" {
            r.service_reference_verified = known("availability_mode") == Some("zonal")
                || absent(value, unknown, "availability_mode");
            continue;
        }
        let Some((kind, output, collection)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !super::vpc_routes::eligible(nodes, r)
            || (!collection && r.sources.len() != 1)
            || if output == "id" {
                !r.ids_only
            } else {
                r.source_attributes.len() != 1 || !r.source_attributes.contains(output)
            }
        {
            continue;
        }
        let other = match (node.resource_type.as_str(), r.attribute.as_str()) {
            ("aws_eip", "instance") => Some("network_interface"),
            ("aws_eip", "network_interface") => Some("instance"),
            ("aws_eip_association", "instance_id") => Some("network_interface_id"),
            ("aws_eip_association", "network_interface_id") => Some("instance_id"),
            _ => None,
        };
        if other.is_some_and(|field| !absent(value, unknown, field)) {
            continue;
        }
        if node.resource_type == "aws_eip" && known("domain") != Some("vpc") {
            continue;
        }
        if node.resource_type == "aws_nat_gateway" {
            let mode = known("availability_mode");
            if !matches!(known("connectivity_type"), Some("public" | "private")) {
                continue;
            }
            let regional = matches!(
                r.attribute.as_str(),
                "vpc_id" | "availability_zone_address.allocation_ids"
            );
            if regional {
                if mode != Some("regional")
                    || known("connectivity_type") != Some("public")
                    || !absent(value, unknown, "subnet_id")
                    || !absent(value, unknown, "allocation_id")
                    || !absent(value, unknown, "secondary_allocation_ids")
                {
                    continue;
                }
            } else if mode != Some("zonal")
                || known("connectivity_type") != Some("public")
                || !absent(value, unknown, "availability_zone_address")
            {
                continue;
            }
        }
        let actual = if r.attribute == "availability_zone_address.allocation_ids" {
            regional_values(value, unknown)
        } else {
            super::streaming::values(value, unknown, &r.attribute, collection)
        };
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                if nodes[s].resource_type != kind {
                    return None;
                }
                if kind == "aws_eip"
                    && super::service_evidence::known(evidence, s, "domain") != Some("vpc")
                {
                    return None;
                }
                if node.resource_type == "aws_nat_gateway_eip_association"
                    && kind == "aws_nat_gateway"
                    && (super::service_evidence::known(evidence, s, "availability_mode")
                        != Some("zonal")
                        || super::service_evidence::known(evidence, s, "connectivity_type")
                            != Some("public"))
                {
                    return None;
                }
                super::service_evidence::known(evidence, s, output)
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = actual.is_some()
            && actual == expected
            && expected
                .as_ref()
                .is_some_and(|s| s.len() == r.sources.len());
    }
}

pub(super) fn association(raw: &Input, target: usize) -> Option<(usize, usize)> {
    let get = |field| {
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == field);
        let r = refs.next()?;
        if refs.next().is_some() || !super::service_references::verified(raw, r) {
            return None;
        }
        match r.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    match raw.nodes[target].resource_type.as_str() {
        "aws_internet_gateway_attachment" => get("internet_gateway_id").zip(get("vpc_id")),
        "aws_nat_gateway_eip_association" => get("nat_gateway_id").zip(get("allocation_id")),
        "aws_eip_association" => {
            let endpoint = match (get("instance_id"), get("network_interface_id")) {
                (Some(s), None) | (None, Some(s)) => s,
                _ => return None,
            };
            get("allocation_id").map(|eip| (eip, endpoint))
        }
        _ => None,
    }
}
