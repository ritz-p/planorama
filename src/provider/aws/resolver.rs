use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

const ENDPOINT: &str = "aws_route53_resolver_endpoint";
const SUBNETS: &str = "ip_address.subnet_id";

fn eligible(nodes: &[TerraformEntity], r: &AttributeReference) -> bool {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == crate::model::EntityMode::Managed
            && n.action != crate::model::Action::Delete
            && n.deposed_key.is_none()
    };
    current(&nodes[r.target])
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && !r.sources.is_empty()
        && r.sources.iter().all(|&s| {
            s != r.target
                && current(&nodes[s])
                && super::route53::configured(&nodes[r.target], &nodes[s])
                && super::scope::scope(&nodes[r.target])
                    .and_then(|s| s.region)
                    .is_some()
                && super::scope::scope(&nodes[r.target]).and_then(|s| s.region)
                    == super::scope::scope(&nodes[s]).and_then(|s| s.region)
        })
}

fn rule_owner(
    evidence: &[(&Value, &Value)],
    index: usize,
) -> Option<Option<crate::model::AwsAccountScope>> {
    let (v, m) = evidence[index];
    let owner = super::service_evidence::known(evidence, index, "owner_id");
    let share = super::service_evidence::known(evidence, index, "share_status");
    if !super::vpc_gateways::absent(v, m, "share_status")
        && !matches!(share, Some("NOT_SHARED" | "SHARED_BY_ME"))
    {
        return None;
    }
    if super::vpc_gateways::absent(v, m, "owner_id") {
        return (share != Some("SHARED_BY_ME")).then_some(None);
    }
    Some(Some(crate::model::AwsAccountScope::new(owner?)?))
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    super::streaming::bindings(raw, r)
        && r.service_reference_owner_account
            .as_ref()
            .is_none_or(|owner| {
                let index = if r.attribute == "resolver_endpoint_id" {
                    r.target
                } else {
                    let Some(&source) = r.sources.first() else {
                        return false;
                    };
                    source
                };
                raw.nodes[index]
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
                    == Some(owner)
            })
}

pub(crate) fn indexed(kind: &str, field: &str) -> bool {
    let parts: Vec<_> = field.split('.').collect();
    kind == ENDPOINT
        && matches!(parts.as_slice(), ["ip_address", i, "subnet_id"] if i.parse::<usize>().is_ok())
}

fn network(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if !eligible(nodes, r) || r.source_attributes != ["id".to_owned()].into() {
        return false;
    }
    let (v, m) = evidence[r.target];
    let mut expected = BTreeSet::new();
    let kind = if r.attribute == SUBNETS {
        "aws_subnet"
    } else {
        "aws_security_group"
    };
    let mut vpc = None;
    for &s in &r.sources {
        if nodes[s].resource_type != kind {
            return false;
        }
        let Some(id) = super::service_evidence::known(evidence, s, "id") else {
            return false;
        };
        let Some(parent) = super::service_evidence::known(evidence, s, "vpc_id") else {
            return false;
        };
        if vpc.is_some_and(|p| p != parent) || !expected.insert(id) {
            return false;
        }
        vpc = Some(parent);
    }
    if !super::vpc_gateways::absent(v, m, "host_vpc_id")
        && super::service_evidence::known(evidence, r.target, "host_vpc_id") != vpc
    {
        return false;
    }
    if r.attribute != SUBNETS {
        return super::streaming::values(v, m, "security_group_ids", true).as_ref()
            == Some(&expected);
    }
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let Some(rows) = v["ip_address"].as_array().filter(|v| !v.is_empty()) else {
        return false;
    };
    let masks = match &m["ip_address"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == rows.len() => Some(a),
        _ => return false,
    };
    let mut actual = BTreeSet::new();
    let mut proven = BTreeSet::new();
    for (i, row) in rows.iter().enumerate() {
        let mask = masks.map_or(&Value::Null, |m| &m[i]);
        let Some(id) = super::streaming::values(row, mask, "subnet_id", false) else {
            return false;
        };
        actual.extend(id);
        let field = format!("ip_address.{i}.subnet_id");
        let mut candidates = refs
            .iter()
            .filter(|a| a.target == r.target && a.attribute == field);
        let Some(a) = candidates.next() else {
            return false;
        };
        let Some(s) = super::route53::endpoint(nodes, a, "aws_subnet", &["id"]) else {
            return false;
        };
        if candidates.next().is_some()
            || a.indexed_block_lengths != [rows.len()]
            || !eligible(nodes, a)
        {
            return false;
        }
        let Some(id) = super::service_evidence::known(evidence, s, "id") else {
            return false;
        };
        proven.insert(id);
    }
    actual.len() == rows.len() && actual == expected && proven == expected
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/resolver.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        (ENDPOINT, SUBNETS) => Some("Configured subnets"),
        (ENDPOINT, "security_group_ids") => Some("Configured security groups"),
        ("aws_route53_resolver_rule", "resolver_endpoint_id") => Some("Outbound resolver endpoint"),
        ("aws_route53_resolver_rule_association", "resolver_rule_id") => Some("Resolver rule"),
        ("aws_route53_resolver_rule_association", "vpc_id") => Some("Associated VPC"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let proofs: Vec<_> = refs
        .iter()
        .map(|r| {
            nodes[r.target].resource_type == ENDPOINT
                && matches!(r.attribute.as_str(), SUBNETS | "security_group_ids")
                && network(nodes, refs, evidence, r)
        })
        .collect();
    for (r, proof) in refs.iter_mut().zip(proofs) {
        if nodes[r.target].resource_type == ENDPOINT && label(ENDPOINT, &r.attribute).is_some() {
            r.service_reference_verified = proof;
        }
    }
    for r in refs {
        if nodes[r.target].resource_type == ENDPOINT {
            continue;
        }
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        let kind = match r.attribute.as_str() {
            "resolver_endpoint_id" => "aws_route53_resolver_endpoint",
            "resolver_rule_id" => "aws_route53_resolver_rule",
            "vpc_id" => "aws_vpc",
            _ => continue,
        };
        let Some(source) = super::route53::endpoint(nodes, r, kind, &["id"]) else {
            continue;
        };
        if !eligible(nodes, r) || !super::route53::configured(&nodes[r.target], &nodes[source]) {
            continue;
        }
        let rule = if r.attribute == "resolver_endpoint_id" {
            Some(r.target)
        } else if r.attribute == "resolver_rule_id" {
            Some(source)
        } else {
            None
        };
        if let Some(index) = rule {
            let Some(owner) = rule_owner(evidence, index) else {
                continue;
            };
            r.service_reference_owner_account = owner;
        }
        let actual = super::service_evidence::known(evidence, r.target, &r.attribute);
        if actual.is_none() || actual != super::service_evidence::known(evidence, source, "id") {
            continue;
        }
        if r.attribute == "resolver_endpoint_id"
            && (super::service_evidence::known(evidence, r.target, "rule_type") != Some("FORWARD")
                || super::service_evidence::known(evidence, source, "direction")
                    != Some("OUTBOUND"))
        {
            continue;
        }
        r.service_reference_verified = true;
    }
}
