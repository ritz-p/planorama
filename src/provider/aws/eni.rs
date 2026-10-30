use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/eni.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static str, bool)> {
    Some(match (kind, field) {
        ("aws_network_interface" | "aws_ec2_instance_connect_endpoint", "subnet_id") => {
            ("aws_subnet", false)
        }
        ("aws_network_interface", "security_groups")
        | ("aws_ec2_instance_connect_endpoint", "security_group_ids") => {
            ("aws_security_group", true)
        }
        ("aws_network_interface", "attachment.instance")
        | ("aws_network_interface_attachment", "instance_id") => ("aws_instance", false),
        (
            "aws_network_interface_attachment"
            | "aws_network_interface_permission"
            | "aws_network_interface_sg_attachment",
            "network_interface_id",
        ) => ("aws_network_interface", false),
        ("aws_network_interface_sg_attachment", "security_group_id") => {
            ("aws_security_group", false)
        }
        _ => return None,
    })
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "subnet_id" => "Interface subnet",
        "security_groups" | "security_group_ids" | "security_group_id" => {
            "Interface security groups"
        }
        "attachment.instance" | "instance_id" => "Attached instance",
        _ => "Network interface",
    })
}

pub(super) fn association(raw: &crate::semantic::Input, target: usize) -> Option<(usize, usize)> {
    let other = match raw.nodes[target].resource_type.as_str() {
        "aws_network_interface_attachment" => "instance_id",
        "aws_network_interface_sg_attachment" => "security_group_id",
        _ => return None,
    };
    let get = |field| {
        let mut matches = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == field);
        let r = matches.next()?;
        if matches.next().is_some() || !super::service_references::verified(raw, r) {
            return None;
        }
        match r.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    get("network_interface_id").zip(get(other))
}

fn basic(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    let Some((kind, collection)) = rule(&nodes[r.target].resource_type, &r.attribute) else {
        return false;
    };
    if !super::vpc_routes::eligible(nodes, r)
        || !r.ids_only
        || (!collection && r.sources.len() != 1)
    {
        return false;
    }
    let (value, unknown) = evidence[r.target];
    let actual = super::streaming::values(value, unknown, &r.attribute, collection);
    let expected = r
        .sources
        .iter()
        .map(|&s| {
            (nodes[s].resource_type == kind)
                .then(|| super::service_evidence::known(evidence, s, "id"))
                .flatten()
        })
        .collect::<Option<BTreeSet<_>>>();
    actual.is_some() && actual == expected && expected.is_some_and(|s| s.len() == r.sources.len())
}

fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    field: &str,
) -> Option<usize> {
    let mut matches = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = matches.next()?;
    if matches.next().is_some() || !basic(nodes, r, evidence) {
        return None;
    }
    match r.sources.as_slice() {
        [s] => Some(*s),
        _ => None,
    }
}

fn subnet_vpc<'a>(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&'a Value, &Value)],
    target: usize,
) -> Option<&'a str> {
    let subnet = endpoint(nodes, refs, evidence, target, "subnet_id")?;
    let vpc = super::service_evidence::known(evidence, subnet, "vpc_id")?;
    if nodes[target].resource_type == "aws_ec2_instance_connect_endpoint" {
        let (value, unknown) = evidence[target];
        if !super::vpc_gateways::absent(value, unknown, "vpc_id")
            && super::service_evidence::known(evidence, target, "vpc_id") != Some(vpc)
        {
            return None;
        }
    }
    Some(vpc)
}

fn index(value: &Value, unknown: &Value, field: &str, optional: bool) -> bool {
    if optional && super::vpc_gateways::absent(value, unknown, field) {
        return true;
    }
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&unknown[field], Value::Null | Value::Bool(false))
        && value[field].as_u64().is_some()
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            if !basic(nodes, r, evidence) {
                return false;
            }
            let kind = nodes[r.target].resource_type.as_str();
            let known = |i, key| super::service_evidence::known(evidence, i, key);
            let (value, unknown) = evidence[r.target];
            if kind == "aws_ec2_instance_connect_endpoint"
                && r.attribute == "subnet_id"
                && !super::vpc_gateways::absent(value, unknown, "vpc_id")
                && (known(r.target, "vpc_id").is_none()
                    || known(r.target, "vpc_id") != known(r.sources[0], "vpc_id"))
            {
                return false;
            }
            if kind == "aws_network_interface_attachment"
                && (!index(value, unknown, "device_index", false)
                    || !index(value, unknown, "network_card_index", true))
            {
                return false;
            }
            if r.attribute == "attachment.instance" {
                let Some(entries) = value["attachment"].as_array() else {
                    return false;
                };
                let [entry] = entries.as_slice() else {
                    return false;
                };
                let mask = match &unknown["attachment"] {
                    Value::Null | Value::Bool(false) => &Value::Null,
                    Value::Array(m) if m.len() == 1 => &m[0],
                    _ => return false,
                };
                if !index(entry, mask, "device_index", false)
                    || !index(entry, mask, "network_card_index", true)
                {
                    return false;
                }
            }
            if matches!(
                r.attribute.as_str(),
                "security_groups" | "security_group_ids"
            ) {
                let vpc = subnet_vpc(nodes, refs, evidence, r.target);
                return vpc.is_some() && r.sources.iter().all(|&s| known(s, "vpc_id") == vpc);
            }
            if kind == "aws_network_interface_sg_attachment" {
                let Some(eni) = endpoint(nodes, refs, evidence, r.target, "network_interface_id")
                else {
                    return false;
                };
                let Some(sg) = endpoint(nodes, refs, evidence, r.target, "security_group_id")
                else {
                    return false;
                };
                let vpc = subnet_vpc(nodes, refs, evidence, eni);
                return vpc.is_some() && known(sg, "vpc_id") == vpc;
            }
            if kind == "aws_network_interface_attachment" {
                return endpoint(nodes, refs, evidence, r.target, "network_interface_id").is_some()
                    && endpoint(nodes, refs, evidence, r.target, "instance_id").is_some();
            }
            true
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(verified) {
        if rule(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
        }
    }
}
