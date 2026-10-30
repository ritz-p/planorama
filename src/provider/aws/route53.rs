use crate::model::{
    Action, AttributeReference, EntityMode, ProviderConfiguration, TerraformEntity,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/route53.rs"]
mod tests;

pub(crate) fn indexed(kind: &str, field: &str) -> bool {
    let parts: Vec<_> = field.split('.').collect();
    kind == "aws_route53_zone"
        && matches!(parts.as_slice(), ["vpc", i, "vpc_id"] if i.parse::<usize>().is_ok())
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_route53_zone", "delegation_set_id") => Some("Delegation set"),
        ("aws_route53_zone", "vpc.vpc_id") => Some("Associated VPC"),
        (
            "aws_route53_zone_association" | "aws_route53_vpc_association_authorization",
            "zone_id",
        ) => Some("Hosted zone"),
        ("aws_route53_zone_association", "vpc_id") => Some("Associated VPC"),
        ("aws_route53_vpc_association_authorization", "vpc_id") => Some("Authorized VPC"),
        _ => None,
    }
}

fn compatible(left: Option<&ProviderConfiguration>, right: Option<&ProviderConfiguration>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => {
            !a.account_conflict
                && !b.account_conflict
                && match (&a.account, &b.account) {
                    (Some(a), Some(b)) => a == b,
                    (None, None) => a == b,
                    _ => false,
                }
        }
        _ => false,
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        compatible(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
        ) || (node.resource_type == "aws_route53_zone"
            && r.attribute == "vpc.vpc_id"
            && r.sources.len() == 1
            && node.provider_configuration.is_none()
            && endpoint.provider_configuration.is_none())
    })
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

fn configured(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    [&left.provider_configuration, &right.provider_configuration]
        .iter()
        .all(|c| c.as_ref().is_some_and(|c| !c.account_conflict))
}

fn endpoint(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    kind: &str,
    fields: &[&str],
) -> Option<usize> {
    let [s] = r.sources.as_slice() else {
        return None;
    };
    (current(&nodes[r.target])
        && current(&nodes[*s])
        && *s != r.target
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && nodes[*s].resource_type == kind
        && r.source_attributes.len() == 1
        && fields.iter().any(|f| r.source_attributes.contains(*f)))
    .then_some(*s)
}

fn scalar<'a>(v: &'a Value, m: &Value, key: &str) -> Option<&'a str> {
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&m[key], Value::Null | Value::Bool(false))
    {
        return None;
    }
    v[key].as_str().filter(|s| !s.is_empty())
}

fn rows<'a>(v: &'a Value, m: &'a Value) -> Option<Vec<(&'a Value, &'a Value)>> {
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let values = v["vpc"].as_array().filter(|v| !v.is_empty())?;
    let masks = match &m["vpc"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(m) if m.len() == values.len() => Some(m),
        _ => return None,
    };
    Some(
        values
            .iter()
            .enumerate()
            .map(|(i, v)| (v, masks.map_or(&Value::Null, |m| &m[i])))
            .collect(),
    )
}

fn region(nodes: &[TerraformEntity], target: usize, source: usize, v: &Value, m: &Value) -> bool {
    let actual = super::scope::scope(&nodes[source]).and_then(|s| s.region);
    let expected = scalar(v, m, "vpc_region").map(str::to_owned).or_else(|| {
        super::vpc_gateways::absent(v, m, "vpc_region")
            .then(|| {
                nodes[target]
                    .provider_configuration
                    .as_ref()?
                    .region
                    .as_ref()
                    .map(|r| r.0.clone())
            })
            .flatten()
    });
    actual.is_some() && actual.as_ref().map(|r| &r.0) == expected.as_ref()
}

fn collection(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let (v, m) = evidence[r.target];
    let Some(rows) = rows(v, m) else {
        return false;
    };
    let mut sources = BTreeSet::new();
    for i in 0..rows.len() {
        let field = format!("vpc.{i}.vpc_id");
        let mut found = refs
            .iter()
            .filter(|a| a.target == r.target && a.attribute == field);
        let Some(a) = found.next() else {
            return false;
        };
        let Some(s) = endpoint(nodes, a, "aws_vpc", &["id"]) else {
            return false;
        };
        if found.next().is_some() || a.indexed_block_lengths != [rows.len()] || !sources.insert(s) {
            return false;
        }
        let Some(id) = super::service_evidence::known(evidence, s, "id") else {
            return false;
        };
        let mut matching = rows
            .iter()
            .filter(|(v, m)| scalar(v, m, "vpc_id") == Some(id));
        let Some((row, mask)) = matching.next() else {
            return false;
        };
        if matching.next().is_some() {
            return false;
        }
        let legacy = rows.len() == 1
            && nodes[r.target].provider_configuration.is_none()
            && nodes[s].provider_configuration.is_none()
            && super::vpc_gateways::absent(row, mask, "vpc_region");
        if !legacy
            && (!configured(&nodes[r.target], &nodes[s]) || !region(nodes, r.target, s, row, mask))
        {
            return false;
        }
    }
    sources == r.sources.iter().copied().collect()
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = nodes[r.target].resource_type.as_str();
            if label(kind, &r.attribute).is_none() {
                return false;
            }
            if kind == "aws_route53_zone" && r.attribute == "vpc.vpc_id" {
                return r.issues.is_empty() && collection(nodes, refs, evidence, r);
            }
            let (target, fields): (_, &[&str]) = match r.attribute.as_str() {
                "zone_id" => ("aws_route53_zone", &["zone_id", "id"]),
                "vpc_id" => ("aws_vpc", &["id"]),
                "delegation_set_id" => ("aws_route53_delegation_set", &["id"]),
                _ => return false,
            };
            let Some(s) = endpoint(nodes, r, target, fields) else {
                return false;
            };
            let (v, m) = evidence[r.target];
            let actual = scalar(v, m, &r.attribute);
            configured(&nodes[r.target], &nodes[s])
                && actual.is_some()
                && actual
                    == super::service_evidence::known(
                        evidence,
                        s,
                        r.source_attributes.first().unwrap(),
                    )
                && (r.attribute != "vpc_id" || region(nodes, r.target, s, v, m))
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
            if verified {
                r.complete = true;
            }
        }
    }
}
