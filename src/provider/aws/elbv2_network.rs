use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/elbv2_network.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    super::aliases::canonical(kind) == "aws_lb"
}

pub(crate) fn indexed(kind: &str, field: &str) -> bool {
    let parts: Vec<_> = field.split('.').collect();
    resource(kind)
        && matches!(parts.as_slice(), ["subnet_mapping", i, "subnet_id" | "allocation_id"] if i.parse::<usize>().is_ok())
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    if !resource(kind) {
        return None;
    }
    match field {
        "subnet_mapping.subnet_id" => Some("Mapped subnets"),
        "subnet_mapping.allocation_id" => Some("Mapped Elastic IPs"),
        "ipam_pools.ipv4_ipam_pool_id" => Some("IPv4 allocation pool"),
        "access_logs.bucket" | "connection_logs.bucket" | "health_check_logs.bucket" => {
            Some("Enabled log bucket")
        }
        _ => None,
    }
}

fn scalar<'a>(value: &'a Value, mask: &Value, key: &str) -> Option<&'a str> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&mask[key], Value::Null | Value::Bool(false))
    {
        return None;
    }
    value[key].as_str().filter(|s| !s.is_empty())
}

fn rows<'a>(value: &'a Value, mask: &'a Value, key: &str) -> Option<Vec<(&'a Value, &'a Value)>> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let values = value[key].as_array().filter(|v| !v.is_empty())?;
    let masks = match &mask[key] {
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

fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    field: &str,
    kind: &str,
    outputs: &[&str],
) -> Option<usize> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = found.next()?;
    let [source] = r.sources.as_slice() else {
        return None;
    };
    if found.next().is_some()
        || !super::vpc_routes::eligible(nodes, r)
        || nodes[*source].resource_type != kind
        || r.source_attributes.len() != 1
        || !outputs.iter().any(|s| r.source_attributes.contains(*s))
    {
        return None;
    }
    let output = r.source_attributes.first()?;
    super::service_evidence::known(evidence, *source, output)?;
    Some(*source)
}

fn mappings(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    allocation: bool,
) -> Option<BTreeSet<usize>> {
    let (value, mask) = evidence[target];
    if refs
        .iter()
        .any(|r| r.target == target && r.attribute == "subnets")
    {
        return None;
    }
    let rows = rows(value, mask, "subnet_mapping")?;
    let mut subnets = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    let mut vpc = None;
    for i in 0..rows.len() {
        let field = format!("subnet_mapping.{i}.subnet_id");
        let subnet = endpoint(nodes, refs, evidence, target, &field, "aws_subnet", &["id"])?;
        let reference = refs
            .iter()
            .find(|r| r.target == target && r.attribute == field)?;
        if reference.indexed_block_lengths != [rows.len()] || !subnets.insert(subnet) {
            return None;
        }
        let id = super::service_evidence::known(evidence, subnet, "id")?;
        let mut matching = rows
            .iter()
            .filter(|(v, m)| scalar(v, m, "subnet_id") == Some(id));
        let (row, row_mask) = *matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let subnet_vpc = super::service_evidence::known(evidence, subnet, "vpc_id")?;
        if vpc.is_some_and(|v| v != subnet_vpc) {
            return None;
        }
        vpc = Some(subnet_vpc);
        if !allocation {
            outputs.insert(subnet);
            continue;
        }
        let field = format!("subnet_mapping.{i}.allocation_id");
        if super::vpc_gateways::absent(row, row_mask, "allocation_id") {
            if refs
                .iter()
                .any(|r| r.target == target && r.attribute == field && r.has_references)
            {
                return None;
            }
            continue;
        }
        let eip = endpoint(
            nodes,
            refs,
            evidence,
            target,
            &field,
            "aws_eip",
            &["allocation_id"],
        )?;
        if refs
            .iter()
            .find(|r| r.target == target && r.attribute == field)?
            .indexed_block_lengths
            != [rows.len()]
            || super::service_evidence::known(evidence, eip, "domain") != Some("vpc")
            || scalar(row, row_mask, "allocation_id")
                != super::service_evidence::known(evidence, eip, "allocation_id")
            || !outputs.insert(eip)
        {
            return None;
        }
    }
    if super::service_evidence::known(evidence, target, "vpc_id").is_some_and(|v| Some(v) != vpc) {
        return None;
    }
    (!outputs.is_empty()).then_some(outputs)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_none()
                || !super::vpc_routes::eligible(nodes, r)
            {
                return false;
            }
            let (value, mask) = evidence[r.target];
            let kind = super::service_evidence::known(evidence, r.target, "load_balancer_type");
            if !matches!(kind, Some("application" | "network" | "gateway")) {
                return false;
            }
            if r.attribute.starts_with("subnet_mapping.") {
                let allocation = r.attribute == "subnet_mapping.allocation_id";
                if allocation
                    && (kind != Some("network")
                        || value["internal"] != Value::Bool(false)
                        || !matches!(&mask["internal"], Value::Null | Value::Bool(false)))
                {
                    return false;
                }
                return mappings(nodes, refs, evidence, r.target, allocation)
                    .is_some_and(|s| s == r.sources.iter().copied().collect());
            }
            let Some((block, field)) = r.attribute.split_once('.') else {
                return false;
            };
            let Some(rows) = rows(value, mask, block) else {
                return false;
            };
            let [(row, row_mask)] = rows.as_slice() else {
                return false;
            };
            let (target, outputs) = if block == "ipam_pools" {
                if kind != Some("application") {
                    return false;
                }
                ("aws_vpc_ipam_pool", &["id"][..])
            } else {
                if (block == "access_logs" && kind == Some("gateway"))
                    || (block != "access_logs" && kind != Some("application"))
                    || row["enabled"] != Value::Bool(true)
                    || !matches!(
                        row_mask,
                        Value::Null | Value::Bool(false) | Value::Object(_)
                    )
                    || !matches!(&row_mask["enabled"], Value::Null | Value::Bool(false))
                {
                    return false;
                }
                ("aws_s3_bucket", &["id", "bucket"][..])
            };
            let Some(source) = endpoint(
                nodes,
                refs,
                evidence,
                r.target,
                &r.attribute,
                target,
                outputs,
            ) else {
                return false;
            };
            let Some(output) = r.source_attributes.first() else {
                return false;
            };
            scalar(row, row_mask, field).is_some()
                && scalar(row, row_mask, field)
                    == super::service_evidence::known(evidence, source, output)
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
        }
    }
}
