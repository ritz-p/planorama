use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/network_acl.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static str, bool)> {
    match (kind, field) {
        ("aws_network_acl", "subnet_ids") => Some(("aws_subnet", true)),
        ("aws_network_acl_association", "subnet_id") => Some(("aws_subnet", false)),
        ("aws_network_acl_association" | "aws_network_acl_rule", "network_acl_id") => {
            Some(("aws_network_acl", false))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "subnet_ids" | "subnet_id" => "Associated subnets",
        "network_acl_id" if kind == "aws_network_acl_association" => "Associated network ACL",
        _ => "Network ACL owner",
    })
}

fn identity(value: &Value, unknown: &Value) -> bool {
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && ["rule_number", "egress", "protocol", "rule_action"]
            .iter()
            .all(|key| matches!(&unknown[key], Value::Null | Value::Bool(false)))
        && value["rule_number"]
            .as_u64()
            .is_some_and(|n| (1..=32766).contains(&n))
        && value["egress"].is_boolean()
        && value["protocol"].as_str().is_some_and(|s| {
            matches!(s, "tcp" | "udp" | "icmp" | "icmpv6" | "all")
                || s.parse::<i16>().is_ok_and(|n| (-1..=255).contains(&n))
        })
        && matches!(value["rule_action"].as_str(), Some("allow" | "deny"))
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs.iter_mut() {
        let kind = nodes[r.target].resource_type.as_str();
        let Some((endpoint, collection)) = rule(kind, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if !super::vpc_routes::eligible(nodes, r)
            || !r.ids_only
            || (!collection && r.sources.len() != 1)
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        if kind == "aws_network_acl_rule" && !identity(value, unknown) {
            continue;
        }
        let actual = super::streaming::values(value, unknown, &r.attribute, collection);
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                (nodes[s].resource_type == endpoint)
                    .then(|| super::service_evidence::known(evidence, s, "id"))
                    .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        if actual.is_none()
            || actual != expected
            || expected.as_ref().is_none_or(|s| s.len() != r.sources.len())
        {
            continue;
        }
        if kind == "aws_network_acl" {
            let vpc = super::service_evidence::known(evidence, r.target, "vpc_id");
            if vpc.is_none()
                || r.sources
                    .iter()
                    .any(|&s| super::service_evidence::known(evidence, s, "vpc_id") != vpc)
            {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
    let pairs = refs
        .iter()
        .filter(|r| r.service_reference_verified)
        .filter_map(|r| match r.sources.as_slice() {
            [s] => Some((r.target, r.attribute.clone(), *s)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for r in refs
        .iter_mut()
        .filter(|r| nodes[r.target].resource_type == "aws_network_acl_association")
    {
        let get = |field| {
            let mut candidates = pairs
                .iter()
                .filter(|(t, f, _)| *t == r.target && f == field);
            let (_, _, s) = candidates.next()?;
            candidates.next().is_none().then_some(*s)
        };
        let parent = get("network_acl_id")
            .and_then(|s| super::service_evidence::known(evidence, s, "vpc_id"));
        let subnet =
            get("subnet_id").and_then(|s| super::service_evidence::known(evidence, s, "vpc_id"));
        r.service_reference_verified &= parent.is_some() && parent == subnet;
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
    Some((get("network_acl_id")?, get("subnet_id")?))
}
