use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/security_group_rules.rs"]
mod tests;

fn rule(kind: &str, field: &str) -> Option<(&'static str, bool)> {
    Some(match (kind, field) {
        ("aws_security_group", "ingress.security_groups" | "egress.security_groups") => {
            ("aws_security_group", true)
        }
        ("aws_security_group", "ingress.prefix_list_ids" | "egress.prefix_list_ids") => {
            ("aws_ec2_managed_prefix_list", true)
        }
        (
            "aws_security_group_rule"
            | "aws_vpc_security_group_ingress_rule"
            | "aws_vpc_security_group_egress_rule"
            | "aws_vpc_security_group_rules_exclusive"
            | "aws_vpc_security_group_vpc_association",
            "security_group_id",
        ) => ("aws_security_group", false),
        ("aws_security_group_rule", "source_security_group_id")
        | (
            "aws_vpc_security_group_ingress_rule" | "aws_vpc_security_group_egress_rule",
            "referenced_security_group_id",
        ) => ("aws_security_group", false),
        ("aws_security_group_rule", "prefix_list_ids") => ("aws_ec2_managed_prefix_list", true),
        (
            "aws_vpc_security_group_ingress_rule" | "aws_vpc_security_group_egress_rule",
            "prefix_list_id",
        ) => ("aws_ec2_managed_prefix_list", false),
        ("aws_vpc_security_group_rules_exclusive", "ingress_rule_ids") => {
            ("aws_vpc_security_group_ingress_rule", true)
        }
        ("aws_vpc_security_group_rules_exclusive", "egress_rule_ids") => {
            ("aws_vpc_security_group_egress_rule", true)
        }
        ("aws_vpc_security_group_vpc_association", "vpc_id") => ("aws_vpc", false),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    rule(kind, field).map(|_| match field {
        "security_group_id" => "Security group owner",
        "vpc_id" => "Additional VPC",
        "ingress_rule_ids" => "Exclusive ingress rules",
        "egress_rule_ids" => "Exclusive egress rules",
        "ingress.security_groups" => "Ingress peer security groups",
        "egress.security_groups" => "Egress peer security groups",
        "ingress.prefix_list_ids" => "Ingress prefix lists",
        "egress.prefix_list_ids" => "Egress prefix lists",
        "prefix_list_id" | "prefix_list_ids" => "Rule prefix lists",
        _ => "Rule peer security group",
    })
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    super::vpc_routes::bindings(raw, r)
}

fn values<'a>(
    value: &'a Value,
    unknown: &'a Value,
    field: &str,
    collection: bool,
) -> Option<BTreeSet<&'a str>> {
    if let Some((block, key)) = field.split_once('.') {
        if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
            || !matches!(&unknown[block], Value::Null | Value::Bool(false))
        {
            return None;
        }
        let entries = value[block].as_array()?;
        let mut result = BTreeSet::new();
        for entry in entries {
            if entry[key].is_null() || entry[key].as_array().is_some_and(Vec::is_empty) {
                continue;
            }
            result.extend(super::streaming::values(entry, &Value::Null, key, true)?);
        }
        Some(result)
    } else {
        if collection
            && value[field].as_array().is_some_and(Vec::is_empty)
            && matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
            && (matches!(&unknown[field], Value::Null | Value::Bool(false))
                || unknown[field].as_array().is_some_and(Vec::is_empty))
        {
            return Some(BTreeSet::new());
        }
        super::streaming::values(value, unknown, field, collection)
    }
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
        let rule_set = matches!(r.attribute.as_str(), "ingress_rule_ids" | "egress_rule_ids");
        let output = if rule_set
            && r.source_attributes.len() == 1
            && r.source_attributes.contains("security_group_rule_id")
        {
            "security_group_rule_id"
        } else {
            "id"
        };
        if !super::vpc_routes::eligible(nodes, r)
            || (output == "id" && !r.ids_only)
            || (!collection && r.sources.len() != 1)
        {
            continue;
        }
        let known = |i, key| super::service_evidence::known(evidence, i, key);
        if kind == "aws_security_group_rule"
            && !matches!(known(r.target, "type"), Some("ingress" | "egress"))
        {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        let field = r.attribute.as_str();
        if matches!(
            kind,
            "aws_vpc_security_group_ingress_rule" | "aws_vpc_security_group_egress_rule"
        ) && field != "security_group_id"
            && (known(r.target, "ip_protocol").is_none()
                || [
                    "cidr_ipv4",
                    "cidr_ipv6",
                    "prefix_list_id",
                    "referenced_security_group_id",
                ]
                .iter()
                .filter(|&&key| key != field)
                .any(|key| !super::vpc_gateways::absent(value, unknown, key)))
        {
            continue;
        }
        if kind == "aws_security_group_rule"
            && field == "source_security_group_id"
            && (!matches!(&unknown["self"], Value::Null | Value::Bool(false))
                || !matches!(&value["self"], Value::Null | Value::Bool(false))
                || ["cidr_blocks", "ipv6_cidr_blocks"]
                    .iter()
                    .any(|key| !super::vpc_gateways::absent(value, unknown, key)))
        {
            continue;
        }
        let actual = values(value, unknown, field, collection);
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                if nodes[s].resource_type != endpoint {
                    return None;
                }
                if rule_set
                    && (known(r.target, "security_group_id").is_none()
                        || known(r.target, "security_group_id") != known(s, "security_group_id"))
                {
                    return None;
                }
                known(s, output)
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = actual.is_some()
            && expected
                .as_ref()
                .is_some_and(|s| s.len() == r.sources.len())
            && actual == expected;
    }
    let proven = refs
        .iter()
        .filter(|r| r.service_reference_verified)
        .map(|r| (r.target, r.attribute.clone(), r.sources.clone()))
        .collect::<Vec<_>>();
    let owner = |i| {
        proven
            .iter()
            .find(|(t, f, s)| *t == i && f == "security_group_id" && s.len() == 1)
            .map(|(_, _, s)| s[0])
    };
    for r in refs.iter_mut() {
        let kind = nodes[r.target].resource_type.as_str();
        if kind == "aws_vpc_security_group_rules_exclusive" {
            let (value, unknown) = evidence[r.target];
            let complete = ["ingress_rule_ids", "egress_rule_ids"].iter().all(|field| {
                values(value, unknown, field, true).is_some_and(|ids| {
                    ids.is_empty()
                        || proven.iter().any(|(t, f, sources)| {
                            *t == r.target
                                && f == field
                                && sources
                                    .iter()
                                    .all(|&s| owner(s).is_some() && owner(s) == owner(r.target))
                        })
                })
            });
            r.service_reference_verified &= complete && owner(r.target).is_some();
        } else if kind == "aws_vpc_security_group_vpc_association" {
            r.service_reference_verified &= owner(r.target).is_some()
                && proven
                    .iter()
                    .any(|(t, f, _)| *t == r.target && f == "vpc_id");
        }
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
    Some((get("security_group_id")?, get("vpc_id")?))
}
