use super::vpc_gateways::absent;
use crate::model::{
    Action, AttributeReference, EntityMode, ProviderConfiguration, RegionScope, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/privatelink.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static [&'static str], &'static str, bool)> {
    Some(match (kind, field) {
        ("aws_vpc_endpoint", "subnet_ids" | "subnet_configuration.subnet_id") => {
            (&["aws_subnet"], "id", true)
        }
        ("aws_vpc_endpoint", "route_table_ids") => (&["aws_route_table"], "id", true),
        ("aws_vpc_endpoint", "security_group_ids") => (&["aws_security_group"], "id", true),
        ("aws_vpc_endpoint", "service_name") => {
            (&["aws_vpc_endpoint_service"], "service_name", false)
        }
        ("aws_vpc_endpoint", "resource_configuration_arn") => {
            (&["aws_vpclattice_resource_configuration"], "arn", false)
        }
        ("aws_vpc_endpoint", "service_network_arn") => {
            (&["aws_vpclattice_service_network"], "arn", false)
        }
        (
            "aws_vpc_endpoint_service",
            "network_load_balancer_arns" | "gateway_load_balancer_arns",
        ) => (&["aws_lb"], "arn", true),
        ("aws_vpc_endpoint_service", "allowed_principals") => {
            (&["aws_iam_role", "aws_iam_user"], "arn", true)
        }
        ("aws_vpc_endpoint_service_allowed_principal", "principal_arn") => {
            (&["aws_iam_role", "aws_iam_user"], "arn", false)
        }
        (
            "aws_vpc_endpoint_connection_accepter"
            | "aws_vpc_endpoint_connection_notification"
            | "aws_vpc_endpoint_service_allowed_principal",
            "vpc_endpoint_service_id",
        )
        | ("aws_vpc_endpoint_service_private_dns_verification", "service_id") => {
            (&["aws_vpc_endpoint_service"], "id", false)
        }
        (
            "aws_vpc_endpoint_connection_accepter"
            | "aws_vpc_endpoint_connection_notification"
            | "aws_vpc_endpoint_policy"
            | "aws_vpc_endpoint_private_dns"
            | "aws_vpc_endpoint_route_table_association"
            | "aws_vpc_endpoint_security_group_association"
            | "aws_vpc_endpoint_subnet_association",
            "vpc_endpoint_id",
        ) => (&["aws_vpc_endpoint"], "id", false),
        ("aws_vpc_endpoint_connection_notification", "connection_notification_arn") => {
            (&["aws_sns_topic"], "arn", false)
        }
        ("aws_vpc_endpoint_route_table_association", "route_table_id") => {
            (&["aws_route_table"], "id", false)
        }
        ("aws_vpc_endpoint_security_group_association", "security_group_id") => {
            (&["aws_security_group"], "id", false)
        }
        ("aws_vpc_endpoint_subnet_association", "subnet_id") => (&["aws_subnet"], "id", false),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "subnet_ids" | "subnet_configuration.subnet_id" | "subnet_id" => "Endpoint subnets",
        "route_table_ids" | "route_table_id" => "Endpoint route tables",
        "security_group_ids" | "security_group_id" => "Endpoint security groups",
        "allowed_principals" | "principal_arn" => "Allowed principals",
        "network_load_balancer_arns" | "gateway_load_balancer_arns" => "Service load balancers",
        "connection_notification_arn" => "Notification topic",
        "vpc_endpoint_id" => "Configured endpoint",
        _ => "Configured service",
    })
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

fn bound(
    kind: &str,
    field: &str,
    left: Option<&ProviderConfiguration>,
    right: Option<&ProviderConfiguration>,
    lr: Option<&RegionScope>,
    rr: Option<&RegionScope>,
    defer_accounts: bool,
) -> bool {
    let principal = matches!(field, "allowed_principals" | "principal_arn");
    let cross = principal
        || (kind == "aws_vpc_endpoint"
            && matches!(
                field,
                "service_name" | "resource_configuration_arn" | "service_network_arn"
            ))
        || (kind == "aws_vpc_endpoint_connection_accepter" && field == "vpc_endpoint_id");
    if !cross {
        return super::eventbridge::bindings(left, right, lr, rr, false);
    }
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    !left.account_conflict
        && !right.account_conflict
        && lr.is_some()
        && (principal || rr.is_some())
        && (defer_accounts || (left.account.is_some() && right.account.is_some()) || left == right)
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let n = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let e = &raw.nodes[s];
        bound(
            &n.resource_type,
            &r.attribute,
            n.provider_configuration.as_ref(),
            e.provider_configuration.as_ref(),
            n.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            e.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            false,
        )
    })
}

fn network_mode(mode: Option<&str>, field: &str) -> bool {
    match field {
        "route_table_ids" | "route_table_id" => mode == Some("Gateway"),
        "security_group_ids" | "security_group_id" => {
            matches!(mode, Some("Interface" | "Resource" | "ServiceNetwork"))
        }
        _ => matches!(
            mode,
            Some("Interface" | "GatewayLoadBalancer" | "Resource" | "ServiceNetwork")
        ),
    }
}

fn service_region(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    endpoint: usize,
    service: usize,
) -> bool {
    let region = super::scope::scope(&nodes[service]).and_then(|s| s.region);
    let (value, unknown) = evidence[endpoint];
    let expected = super::service_evidence::known(evidence, endpoint, "service_region");
    if let Some(expected) = expected {
        region.is_some_and(|r| r.0 == expected)
    } else {
        absent(value, unknown, "service_region")
            && region.is_some()
            && region == super::scope::scope(&nodes[endpoint]).and_then(|s| s.region)
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs.iter_mut() {
        let n = &nodes[r.target];
        let Some((kinds, output, collection)) = rule(&n.resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !current(n)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || (!collection && r.sources.len() != 1)
            || if output == "id" {
                !r.ids_only
            } else {
                r.source_attributes.len() != 1 || !r.source_attributes.contains(output)
            }
        {
            continue;
        }
        let known = |i, key| super::service_evidence::known(evidence, i, key);
        let (value, unknown) = evidence[r.target];
        let field = r.attribute.as_str();
        let mode = known(r.target, "vpc_endpoint_type");
        let network = n.resource_type == "aws_vpc_endpoint"
            && matches!(
                field,
                "subnet_ids"
                    | "subnet_configuration.subnet_id"
                    | "route_table_ids"
                    | "security_group_ids"
            );
        if network
            && (!network_mode(mode, field)
                || (mode == Some("GatewayLoadBalancer") && r.sources.len() != 1))
        {
            continue;
        }
        if n.resource_type == "aws_vpc_endpoint"
            && matches!(
                field,
                "service_name" | "resource_configuration_arn" | "service_network_arn"
            )
        {
            if ![
                "service_name",
                "resource_configuration_arn",
                "service_network_arn",
            ]
            .iter()
            .filter(|&&key| key != field)
            .all(|key| absent(value, unknown, key))
            {
                continue;
            }
            if match field {
                "service_name" => !matches!(mode, Some("Interface" | "GatewayLoadBalancer")),
                "resource_configuration_arn" => mode != Some("Resource"),
                _ => mode != Some("ServiceNetwork"),
            } {
                continue;
            }
        }
        if n.resource_type == "aws_vpc_endpoint_connection_notification"
            && matches!(field, "vpc_endpoint_id" | "vpc_endpoint_service_id")
        {
            let other = if field == "vpc_endpoint_id" {
                "vpc_endpoint_service_id"
            } else {
                "vpc_endpoint_id"
            };
            if !absent(value, unknown, other) {
                continue;
            }
        }
        let actual = if field == "subnet_configuration.subnet_id" {
            if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                || !matches!(
                    &unknown["subnet_configuration"],
                    Value::Null | Value::Bool(false)
                )
            {
                continue;
            }
            let Some(entries) = value["subnet_configuration"].as_array() else {
                continue;
            };
            let ids = entries
                .iter()
                .map(|e| e["subnet_id"].as_str().filter(|s| !s.is_empty()))
                .collect::<Option<BTreeSet<_>>>();
            let subnets = super::streaming::values(value, unknown, "subnet_ids", true);
            if !ids
                .as_ref()
                .zip(subnets.as_ref())
                .is_some_and(|(ids, subnets)| ids.len() == entries.len() && ids.is_subset(subnets))
            {
                continue;
            }
            ids
        } else {
            super::streaming::values(value, unknown, field, collection)
        };
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                let e = &nodes[s];
                if s == r.target
                    || !current(e)
                    || !kinds.contains(&e.resource_type.as_str())
                    || !bound(
                        &n.resource_type,
                        field,
                        n.provider_configuration.as_ref(),
                        e.provider_configuration.as_ref(),
                        super::scope::scope(n)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(e)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        true,
                    )
                {
                    return None;
                }
                if network
                    && (known(r.target, "vpc_id").is_none()
                        || known(r.target, "vpc_id") != known(s, "vpc_id"))
                {
                    return None;
                }
                if n.resource_type == "aws_vpc_endpoint"
                    && field == "service_name"
                    && (!service_region(nodes, evidence, r.target, s)
                        || mode != known(s, "service_type"))
                {
                    return None;
                }
                if n.resource_type == "aws_vpc_endpoint"
                    && matches!(field, "resource_configuration_arn" | "service_network_arn")
                    && super::scope::scope(n).and_then(|s| s.region)
                        != super::scope::scope(e).and_then(|s| s.region)
                {
                    return None;
                }
                if n.resource_type == "aws_vpc_endpoint_service"
                    && matches!(
                        field,
                        "network_load_balancer_arns" | "gateway_load_balancer_arns"
                    )
                {
                    let (lb, service, other) = if field == "network_load_balancer_arns" {
                        ("network", "Interface", "gateway_load_balancer_arns")
                    } else {
                        (
                            "gateway",
                            "GatewayLoadBalancer",
                            "network_load_balancer_arns",
                        )
                    };
                    if known(s, "load_balancer_type") != Some(lb)
                        || known(r.target, "service_type") != Some(service)
                        || !absent(value, unknown, other)
                    {
                        return None;
                    }
                }
                if field == "vpc_endpoint_id" {
                    let endpoint_mode = known(s, "vpc_endpoint_type");
                    let allowed = match n.resource_type.as_str() {
                        "aws_vpc_endpoint_private_dns" => endpoint_mode == Some("Interface"),
                        "aws_vpc_endpoint_policy" => {
                            matches!(endpoint_mode, Some("Gateway" | "Interface"))
                        }
                        "aws_vpc_endpoint_route_table_association" => {
                            network_mode(endpoint_mode, "route_table_id")
                        }
                        "aws_vpc_endpoint_security_group_association" => {
                            network_mode(endpoint_mode, "security_group_id")
                        }
                        "aws_vpc_endpoint_subnet_association" => {
                            network_mode(endpoint_mode, "subnet_id")
                        }
                        "aws_vpc_endpoint_connection_accepter" => {
                            matches!(endpoint_mode, Some("Interface" | "GatewayLoadBalancer"))
                        }
                        _ => true,
                    };
                    if !allowed {
                        return None;
                    }
                }
                let result = known(s, output)?;
                if matches!(field, "allowed_principals" | "principal_arn") {
                    let parts = result.split(':').collect::<Vec<_>>();
                    let prefix = if e.resource_type == "aws_iam_role" {
                        "role/"
                    } else {
                        "user/"
                    };
                    if parts.len() != 6
                        || parts[0] != "arn"
                        || parts[2] != "iam"
                        || !parts[3].is_empty()
                        || parts[4].len() != 12
                        || !parts[4].bytes().all(|b| b.is_ascii_digit())
                        || !parts[5].starts_with(prefix)
                        || parts[5].len() == prefix.len()
                        || result.contains('*')
                    {
                        return None;
                    }
                }
                Some(result)
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = actual.is_some()
            && actual == expected
            && expected
                .as_ref()
                .is_some_and(|s| s.len() == r.sources.len());
    }
    let proven = refs
        .iter()
        .filter(|r| r.service_reference_verified)
        .filter_map(|r| match r.sources.as_slice() {
            [s] => Some((r.target, r.attribute.clone(), *s)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for r in refs
        .iter_mut()
        .filter(|r| association_fields(&nodes[r.target].resource_type).is_some())
    {
        let fields = association_fields(&nodes[r.target].resource_type).unwrap();
        let get = |field| {
            proven
                .iter()
                .find(|(t, a, _)| *t == r.target && a == field)
                .map(|(_, _, s)| *s)
        };
        let valid = get(fields[0])
            .zip(get(fields[1]))
            .is_some_and(|(endpoint, other)| {
                let known = |i, key| super::service_evidence::known(evidence, i, key);
                if nodes[r.target].resource_type == "aws_vpc_endpoint_connection_accepter" {
                    known(endpoint, "service_name").is_some()
                        && known(endpoint, "service_name") == known(other, "service_name")
                        && known(endpoint, "vpc_endpoint_type") == known(other, "service_type")
                        && service_region(nodes, evidence, endpoint, other)
                } else {
                    known(endpoint, "vpc_id").is_some()
                        && known(endpoint, "vpc_id") == known(other, "vpc_id")
                }
            });
        r.service_reference_verified &= valid;
    }
}

fn association_fields(kind: &str) -> Option<[&'static str; 2]> {
    Some(match kind {
        "aws_vpc_endpoint_connection_accepter" => ["vpc_endpoint_id", "vpc_endpoint_service_id"],
        "aws_vpc_endpoint_subnet_association" => ["vpc_endpoint_id", "subnet_id"],
        "aws_vpc_endpoint_route_table_association" => ["vpc_endpoint_id", "route_table_id"],
        "aws_vpc_endpoint_security_group_association" => ["vpc_endpoint_id", "security_group_id"],
        _ => return None,
    })
}

pub(super) fn association(raw: &Input, index: usize) -> Option<(usize, usize)> {
    let fields = association_fields(&raw.nodes[index].resource_type)?;
    let get = |field| {
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.attribute == field);
        let r = refs.next()?;
        if refs.next().is_some()
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || !super::service_references::verified(raw, r)
        {
            return None;
        }
        match r.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    get(fields[0]).zip(get(fields[1]))
}
