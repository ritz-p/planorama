use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/route53_records.rs"]
mod tests;

fn normal(path: &str) -> String {
    path.split('.')
        .filter(|p| p.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == "aws_route53_records_exclusive"
        && normal(path) != path
        && field(normal(path).trim_start_matches("resource_record_set.")).is_some()
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_route53_records_exclusive", "zone_id") => Some("Hosted zone"),
        ("aws_route53_cidr_location", "cidr_collection_id") => Some("CIDR collection membership"),
        ("aws_route53_traffic_policy_instance", "hosted_zone_id") => Some("Hosted zone"),
        ("aws_route53_traffic_policy_instance", "traffic_policy_id") => {
            Some("Traffic policy version")
        }
        ("aws_route53_record", "health_check_id") => Some("Health check reference"),
        ("aws_route53_record", "cidr_routing_policy.collection_id") => Some("CIDR collection"),
        ("aws_route53_record", "cidr_routing_policy.location_name") => Some("CIDR location"),
        ("aws_route53_records_exclusive", p)
            if !p.split('.').any(|p| p.parse::<usize>().is_ok()) =>
        {
            match p.strip_prefix("resource_record_set.")? {
                "health_check_id" => Some("Health check reference"),
                "traffic_policy_instance_id" => Some("Traffic policy instance"),
                "cidr_routing_config.collection_id" => Some("CIDR collection"),
                "cidr_routing_config.location_name" => Some("CIDR location"),
                "alias_target.dns_name" => Some("Alias target"),
                _ => None,
            }
        }
        _ => None,
    }
}

fn field(path: &str) -> Option<(&'static str, &'static [&'static str])> {
    match path {
        "zone_id" | "hosted_zone_id" => Some(("aws_route53_zone", &["id", "zone_id"])),
        "health_check_id" => Some(("aws_route53_health_check", &["id"])),
        "traffic_policy_instance_id" => Some(("aws_route53_traffic_policy_instance", &["id"])),
        "traffic_policy_id" => Some(("aws_route53_traffic_policy", &["id"])),
        "cidr_collection_id"
        | "cidr_routing_policy.collection_id"
        | "cidr_routing_config.collection_id" => Some(("aws_route53_cidr_collection", &["id"])),
        "cidr_routing_policy.location_name" | "cidr_routing_config.location_name" => {
            Some(("aws_route53_cidr_location", &["name"]))
        }
        "alias_target.dns_name" | "alias_target.hosted_zone_id" => Some(("", &[])),
        _ => None,
    }
}

fn alias_fields(kind: &str, zone: bool) -> &'static [&'static str] {
    match (kind, zone) {
        ("aws_lb" | "aws_elb", false) => &["dns_name"],
        ("aws_lb" | "aws_elb", true) => &["zone_id"],
        ("aws_cloudfront_distribution", false) => &["domain_name"],
        ("aws_cloudfront_distribution" | "aws_globalaccelerator_accelerator", true) => {
            &["hosted_zone_id"]
        }
        ("aws_globalaccelerator_accelerator", false) => &["dns_name", "dual_stack_dns_name"],
        _ => &[],
    }
}

fn reference<'a>(
    refs: &'a [AttributeReference],
    target: usize,
    path: &str,
) -> Option<&'a AttributeReference> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    found.next().is_none().then_some(r)
}

fn source(nodes: &[TerraformEntity], r: &AttributeReference, path: &str) -> Option<usize> {
    let [s] = r.sources.as_slice() else {
        return None;
    };
    let alias = matches!(
        path,
        "alias.name" | "alias.zone_id" | "alias_target.dns_name" | "alias_target.hosted_zone_id"
    );
    let (kind, outputs) = if alias {
        let kind = super::aliases::entity_type(&nodes[*s]);
        (kind, alias_fields(kind, path.ends_with("zone_id")))
    } else {
        field(path)?
    };
    super::route53::endpoint(nodes, r, kind, outputs)
}

fn value_at<'a>(v: &'a Value, m: &Value, path: &[&str]) -> Option<&'a Value> {
    if let Some((key, rest)) = path.split_first() {
        if let Some(values) = v.as_array() {
            let i = key.parse::<usize>().ok()?;
            let mask = match m {
                Value::Null | Value::Bool(false) => &Value::Null,
                Value::Array(ms) if ms.len() == values.len() => ms.get(i)?,
                _ => return None,
            };
            return value_at(values.get(i)?, mask, rest);
        }
        if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        value_at(v.get(*key)?, &m[*key], rest)
    } else {
        matches!(m, Value::Null | Value::Bool(false)).then_some(v)
    }
}

fn output<'a>(
    evidence: &[(&'a Value, &Value)],
    s: usize,
    r: &AttributeReference,
) -> Option<&'a str> {
    super::service_evidence::known(evidence, s, r.source_attributes.first()?)
}

fn matches_value(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    path: &str,
    actual: &Value,
) -> Option<usize> {
    let s = source(nodes, r, path)?;
    if !super::route53::configured(&nodes[r.target], &nodes[s])
        || actual.as_str()? != output(evidence, s, r)?
    {
        return None;
    }
    if path.starts_with("alias")
        && super::aliases::entity_type(&nodes[s]) == "aws_lb"
        && !matches!(
            super::service_evidence::known(evidence, s, "load_balancer_type"),
            Some("application" | "network")
        )
    {
        return None;
    }
    if path.starts_with("alias")
        && matches!(super::aliases::entity_type(&nodes[s]), "aws_lb" | "aws_elb")
        && super::scope::scope(&nodes[s])
            .and_then(|s| s.region)
            .is_none()
    {
        return None;
    }
    Some(s)
}

fn singleton<'a>(v: &'a Value, m: &Value, path: &str) -> Option<&'a Value> {
    let mut v = v;
    let mut m = m;
    for key in path.split('.') {
        if let Some(a) = v.as_array() {
            if a.len() != 1 {
                return None;
            }
            m = match m {
                Value::Null | Value::Bool(false) => &Value::Null,
                Value::Array(a) if a.len() == 1 => &a[0],
                _ => return None,
            };
            v = &a[0];
        }
        if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        v = v.get(key)?;
        m = &m[key];
    }
    matches!(m, Value::Null | Value::Bool(false)).then_some(v)
}

fn paired(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    path: &str,
    actual: &Value,
    (peer, peer_path): (Option<&Value>, &str),
) -> Option<usize> {
    let s = matches_value(nodes, evidence, r, path, actual)?;
    if path == "traffic_policy_instance_id" {
        let owner = reference(refs, r.target, "zone_id")?;
        let instance_owner = reference(refs, s, "hosted_zone_id")?;
        let zone = source(nodes, owner, "zone_id")?;
        if source(nodes, instance_owner, "hosted_zone_id")? != zone
            || super::service_evidence::known(evidence, r.target, "zone_id").is_none()
            || super::service_evidence::known(evidence, r.target, "zone_id")
                != super::service_evidence::known(evidence, s, "hosted_zone_id")
            || super::service_evidence::known(evidence, r.target, "zone_id")
                != output(evidence, zone, owner)
            || super::service_evidence::known(evidence, s, "hosted_zone_id")
                != output(evidence, zone, instance_owner)
        {
            return None;
        }
    } else if path.contains("location_name") {
        let partner = reference(refs, r.target, peer_path)?;
        let c = matches_value(
            nodes,
            evidence,
            partner,
            &normal(peer_path).replace("resource_record_set.", ""),
            peer?,
        )?;
        let membership = reference(refs, s, "cidr_collection_id")?;
        if source(nodes, membership, "cidr_collection_id")? != c
            || super::service_evidence::known(evidence, s, "cidr_collection_id")
                != super::service_evidence::known(evidence, c, "id")
        {
            return None;
        }
    } else if path.starts_with("alias") {
        let partner = reference(refs, r.target, peer_path)?;
        let partner_path = if path == "alias.name" {
            "alias.zone_id"
        } else {
            "alias_target.hosted_zone_id"
        };
        if matches_value(nodes, evidence, partner, partner_path, peer?)? != s {
            return None;
        }
    }
    Some(s)
}

fn ordinary(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let (v, m) = evidence[r.target];
    let Some(actual) = singleton(v, m, &r.attribute) else {
        return false;
    };
    let peer_path = if r.attribute == "alias.name" {
        "alias.zone_id".to_owned()
    } else {
        r.attribute.replace("location_name", "collection_id")
    };
    let Some(s) = paired(
        nodes,
        refs,
        evidence,
        r,
        &r.attribute,
        actual,
        (singleton(v, m, &peer_path), &peer_path),
    ) else {
        return false;
    };
    if r.attribute == "traffic_policy_id" {
        let Some(version) = singleton(v, m, "traffic_policy_version")
            .and_then(Value::as_u64)
            .filter(|v| *v > 0)
        else {
            return false;
        };
        let (sv, sm) = evidence[s];
        if singleton(sv, sm, "version").and_then(Value::as_u64) != Some(version) {
            return false;
        }
        if let Some(version_ref) = reference(refs, r.target, "traffic_policy_version") {
            if version_ref.has_references
                && super::route53::endpoint(
                    nodes,
                    version_ref,
                    "aws_route53_traffic_policy",
                    &["version"],
                ) != Some(s)
            {
                return false;
            }
        }
    }
    true
}

fn leaves<'a>(
    v: &'a Value,
    m: &Value,
    path: String,
    lengths: Vec<usize>,
    out: &mut Vec<(String, Vec<usize>, &'a Value)>,
) -> Option<()> {
    if let Some(values) = v.as_array() {
        let masks = match m {
            Value::Null | Value::Bool(false) => None,
            Value::Array(a) if a.len() == values.len() => Some(a),
            _ => return None,
        };
        for (i, v) in values.iter().enumerate() {
            let mut lengths = lengths.clone();
            lengths.push(values.len());
            leaves(
                v,
                masks.map_or(&Value::Null, |a| &a[i]),
                format!("{path}.{i}"),
                lengths,
                out,
            )?;
        }
    } else if let Some(obj) = v.as_object() {
        if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        for (key, v) in obj {
            let next = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            let normalized = normal(&next);
            if field(&normalized).is_some()
                || ["alias_target", "cidr_routing_config"].contains(&normalized.as_str())
            {
                if v.is_null() && matches!(&m[key], Value::Null | Value::Bool(false)) {
                    continue;
                }
                leaves(v, &m[key], next, lengths.clone(), out)?;
            }
        }
        if m.as_object().is_some_and(|m| {
            m.iter()
                .any(|(k, m)| !obj.contains_key(k) && m != &Value::Bool(false) && !m.is_null())
        }) {
            return None;
        }
    } else {
        if !matches!(m, Value::Null | Value::Bool(false)) {
            return None;
        }
        out.push((path, lengths, v));
    }
    Some(())
}

fn exclusive(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let (v, m) = evidence[r.target];
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let Some(rows) = v["resource_record_set"]
        .as_array()
        .filter(|a| !a.is_empty())
    else {
        return false;
    };
    let masks = match &m["resource_record_set"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == rows.len() => Some(a),
        _ => return false,
    };
    let mut used = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for i in 0..rows.len() {
        let prefix = format!("resource_record_set.{i}.");
        let configured: Vec<_> = refs
            .iter()
            .filter(|a| {
                a.target == r.target
                    && a.attribute.starts_with(&prefix)
                    && indexed("aws_route53_records_exclusive", &a.attribute)
            })
            .collect();
        let mut candidates = Vec::new();
        for (j, row) in rows.iter().enumerate() {
            let mask = masks.map_or(&Value::Null, |a| &a[j]);
            let mut values = Vec::new();
            if leaves(row, mask, String::new(), vec![rows.len()], &mut values).is_none()
                || values.len() != configured.len()
            {
                continue;
            }
            let mut matched = Vec::new();
            let ok = values.iter().all(|(path, lengths, value)| {
                let full = format!("{prefix}{path}");
                let Some(a) = reference(refs, r.target, &full) else {
                    return false;
                };
                if a.indexed_block_lengths != *lengths {
                    return false;
                }
                let normalized = normal(path);
                if normalized == "alias_target.hosted_zone_id" {
                    return matches_value(nodes, evidence, a, &normalized, value).is_some();
                }
                let peer = path
                    .replace("location_name", "collection_id")
                    .replace("dns_name", "hosted_zone_id");
                let Some(s) = paired(
                    nodes,
                    refs,
                    evidence,
                    a,
                    &normalized,
                    value,
                    (
                        value_at(row, mask, &peer.split('.').collect::<Vec<_>>()),
                        &format!("{prefix}{peer}"),
                    ),
                ) else {
                    return false;
                };
                if normal(&full) == r.attribute {
                    matched.push(s);
                }
                true
            });
            if ok {
                candidates.push((j, matched));
            }
        }
        let [(j, matched)] = candidates.as_slice() else {
            return false;
        };
        if !used.insert(*j) {
            return false;
        }
        sources.extend(matched.iter().copied());
    }
    !sources.is_empty() && sources == r.sources.iter().copied().collect()
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    if raw.nodes[r.target].resource_type == "aws_route53_records_exclusive"
        && r.attribute != "zone_id"
        && !reference(&raw.attributes, r.target, "zone_id")
            .is_some_and(|a| super::service_references::verified(raw, a))
    {
        return false;
    }
    if raw.nodes[r.target].resource_type == "aws_route53_records_exclusive"
        && r.attribute != "zone_id"
        && raw
            .attributes
            .iter()
            .filter(|a| {
                a.target == r.target && indexed("aws_route53_records_exclusive", &a.attribute)
            })
            .any(|a| {
                !super::route53::bindings(raw, a)
                    || a.sources.iter().any(|&s| {
                        raw.nodes[s].resource_type == "aws_route53_traffic_policy_instance"
                            && !reference(&raw.attributes, s, "hosted_zone_id")
                                .is_some_and(|a| super::service_references::verified(raw, a))
                    })
                    || a.sources.iter().any(|&s| {
                        raw.nodes[s].resource_type == "aws_route53_cidr_location"
                            && !reference(&raw.attributes, s, "cidr_collection_id")
                                .is_some_and(|m| super::service_references::verified(raw, m))
                    })
            })
    {
        return false;
    }
    super::route53::bindings(raw, r)
        && r.sources.iter().all(|&s| {
            if raw.nodes[s].resource_type != "aws_route53_cidr_location" {
                return true;
            }
            reference(&raw.attributes, s, "cidr_collection_id")
                .is_some_and(|a| super::service_references::verified(raw, a))
        })
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = &nodes[r.target].resource_type;
            if kind == "aws_route53_record" && r.attribute == "alias.name" {
                let additional = r.sources.iter().any(|&s| {
                    matches!(
                        nodes[s].resource_type.as_str(),
                        "aws_elb" | "aws_globalaccelerator_accelerator"
                    )
                });
                return additional.then(|| ordinary(nodes, refs, evidence, r));
            }
            label(kind, &r.attribute)?;
            Some(
                if kind == "aws_route53_records_exclusive" && r.attribute != "zone_id" {
                    exclusive(nodes, refs, evidence, r)
                } else {
                    ordinary(nodes, refs, evidence, r)
                },
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(verified) = result {
            r.service_reference_verified = verified;
            if verified {
                r.complete = true;
            }
        }
    }
}
