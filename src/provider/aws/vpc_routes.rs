use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/vpc_routes.rs"]
mod tests;

fn rule(resource: &str, attribute: &str) -> Option<(&'static [&'static str], &'static str, bool)> {
    match (resource, attribute) {
        ("aws_route", "destination_prefix_list_id")
        | ("aws_route_table", "route.destination_prefix_list_id") => {
            Some((&["aws_ec2_managed_prefix_list"], "id", false))
        }
        ("aws_route", "vpc_endpoint_id") | ("aws_route_table", "route.vpc_endpoint_id") => {
            Some((&["aws_vpc_endpoint"], "id", false))
        }
        ("aws_route", "core_network_arn") | ("aws_route_table", "route.core_network_arn") => {
            Some((&["aws_networkmanager_core_network"], "arn", false))
        }
        ("aws_route", "odb_network_arn") | ("aws_route_table", "route.odb_network_arn") => {
            Some((&["aws_odb_network"], "arn", false))
        }
        ("aws_route_table", "propagating_vgws") => Some((&["aws_vpn_gateway"], "id", true)),
        ("aws_main_route_table_association" | "aws_route_table_association", "route_table_id") => {
            Some((&["aws_route_table"], "id", false))
        }
        ("aws_main_route_table_association", "vpc_id") => Some((&["aws_vpc"], "id", false)),
        ("aws_route_table_association", "subnet_id") => Some((&["aws_subnet"], "id", false)),
        ("aws_route_table_association", "gateway_id") => {
            Some((&["aws_internet_gateway", "aws_vpn_gateway"], "id", false))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|_| match attribute {
        "propagating_vgws" => "Propagating VPN gateways",
        "destination_prefix_list_id" | "route.destination_prefix_list_id" => {
            "Destination prefix list"
        }
        "route_table_id" => "Associated route table",
        "subnet_id" => "Associated subnet",
        "gateway_id" => "Associated gateway",
        "vpc_id" => "Main table VPC",
        _ => "Route target",
    })
}

fn absent(value: &Value, unknown: &Value, field: &str) -> bool {
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&unknown[field], Value::Null | Value::Bool(false))
        && (value[field].is_null() || value[field].as_str() == Some(""))
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_networkmanager_core_network",
        )
    })
}

fn eligible(nodes: &[TerraformEntity], r: &AttributeReference) -> bool {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == crate::model::EntityMode::Managed
            && n.action != crate::model::Action::Delete
            && n.deposed_key.is_none()
    };
    let node = &nodes[r.target];
    current(node)
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && !r.sources.is_empty()
        && r.sources.iter().all(|&s| {
            let endpoint = &nodes[s];
            s != r.target
                && current(endpoint)
                && super::eventbridge::bindings(
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    super::scope::scope(node)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    super::scope::scope(endpoint)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    endpoint.resource_type == "aws_networkmanager_core_network",
                )
        })
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut target_counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        let attribute = if nodes[r.target].resource_type == "aws_route_table" {
            r.attribute.strip_prefix("route.").unwrap_or("")
        } else if nodes[r.target].resource_type == "aws_route" {
            &r.attribute
        } else {
            ""
        };
        if matches!(
            attribute,
            "gateway_id"
                | "nat_gateway_id"
                | "transit_gateway_id"
                | "vpc_peering_connection_id"
                | "network_interface_id"
                | "egress_only_gateway_id"
                | "carrier_gateway_id"
                | "local_gateway_id"
                | "vpc_endpoint_id"
                | "core_network_arn"
                | "odb_network_arn"
        ) {
            *target_counts.entry(r.target).or_insert(0usize) += 1;
        }
    }
    for r in refs.iter_mut() {
        let node = &nodes[r.target];
        let Some((kinds, field, collection)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if (r.attribute.ends_with("vpc_endpoint_id")
            || r.attribute.ends_with("core_network_arn")
            || r.attribute.ends_with("odb_network_arn"))
            && target_counts.get(&r.target) != Some(&1)
        {
            continue;
        }
        let output_proven = if field == "id" {
            r.ids_only
        } else {
            r.source_attributes.len() == 1 && r.source_attributes.contains(field)
        };
        if !eligible(nodes, r)
            || !output_proven
            || (!collection && !r.attribute.starts_with("route.") && r.sources.len() != 1)
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let actual = if let Some(key) = r.attribute.strip_prefix("route.") {
            let Some(entries) = value["route"].as_array() else {
                continue;
            };
            if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                || !matches!(&unknown["route"], Value::Null | Value::Bool(false))
            {
                continue;
            }
            entries
                .iter()
                .map(|entry| entry[key].as_str().filter(|s| !s.is_empty()))
                .collect::<Option<BTreeSet<_>>>()
        } else {
            super::streaming::values(value, unknown, &r.attribute, collection)
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                if !kinds.contains(&nodes[source].resource_type.as_str()) {
                    return None;
                }
                if r.attribute.ends_with("vpc_endpoint_id")
                    && super::service_evidence::known(evidence, source, "vpc_endpoint_type")
                        != Some("GatewayLoadBalancer")
                {
                    return None;
                }
                super::service_evidence::known(evidence, source, field)
            })
            .collect::<Option<BTreeSet<_>>>();
        if actual.is_none()
            || expected.as_ref().is_none_or(|s| s.len() != r.sources.len())
            || actual != expected
        {
            continue;
        }
        if node.resource_type == "aws_route_table_association" {
            let other = match r.attribute.as_str() {
                "subnet_id" => "gateway_id",
                "gateway_id" => "subnet_id",
                _ => "",
            };
            if !other.is_empty() && !absent(value, unknown, other) {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
    let valid = refs
        .iter()
        .filter_map(|r| match r.sources.as_slice() {
            [source]
                if r.service_reference_verified
                    && matches!(
                        nodes[r.target].resource_type.as_str(),
                        "aws_route_table_association" | "aws_main_route_table_association"
                    ) =>
            {
                Some((r.target, r.attribute.clone(), *source))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for r in refs.iter_mut().filter(|r| {
        matches!(
            nodes[r.target].resource_type.as_str(),
            "aws_route_table_association" | "aws_main_route_table_association"
        )
    }) {
        let find = |attribute| {
            valid
                .iter()
                .find(|(target, key, _)| *target == r.target && key == attribute)
                .map(|(_, _, source)| *source)
        };
        let Some(table) = find("route_table_id") else {
            r.service_reference_verified = false;
            continue;
        };
        let parent = super::service_evidence::known(evidence, table, "vpc_id");
        let endpoint_parent = if nodes[r.target].resource_type == "aws_main_route_table_association"
        {
            find("vpc_id").and_then(|s| super::service_evidence::known(evidence, s, "id"))
        } else {
            find("subnet_id")
                .or_else(|| find("gateway_id"))
                .and_then(|s| super::service_evidence::known(evidence, s, "vpc_id"))
        };
        r.service_reference_verified &= parent.is_some() && parent == endpoint_parent;
    }
}

pub(super) fn association(raw: &Input, target: usize) -> Option<(usize, usize)> {
    let get = |attribute| {
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == attribute);
        let r = refs.next()?;
        if refs.next().is_some() || !super::service_references::verified(raw, r) {
            return None;
        }
        match r.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    let table = get("route_table_id")?;
    let endpoint = if raw.nodes[target].resource_type == "aws_main_route_table_association" {
        get("vpc_id")?
    } else {
        match (get("subnet_id"), get("gateway_id")) {
            (Some(s), None) | (None, Some(s)) => s,
            _ => return None,
        }
    };
    Some((endpoint, table))
}
