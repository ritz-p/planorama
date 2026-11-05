use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/vpc_controls.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_vpc_block_public_access_exclusion", "vpc_id" | "subnet_id") => {
            Some("Exclusion target")
        }
        ("aws_vpc_encryption_control", "vpc_id") => Some("Encryption control VPC"),
        ("aws_vpc_dhcp_options_association", "vpc_id") => Some("Associated VPC"),
        ("aws_vpc_dhcp_options_association", "dhcp_options_id") => Some("DHCP options"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !super::service_evidence::eligible(nodes, r)
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains("id")
        {
            continue;
        }
        let source = r.sources[0];
        let endpoint = match r.attribute.as_str() {
            "vpc_id" => "aws_vpc",
            "subnet_id" => "aws_subnet",
            _ => "aws_vpc_dhcp_options",
        };
        if nodes[source].resource_type != endpoint {
            continue;
        }
        let known = |path| super::service_evidence::known(evidence, r.target, path);
        let Some(actual) = known(&r.attribute) else {
            continue;
        };
        if actual == "default"
            || super::service_evidence::known(evidence, source, "id") != Some(actual)
        {
            continue;
        }
        let (v, m) = evidence[r.target];
        if kind == "aws_vpc_block_public_access_exclusion" {
            let other = if r.attribute == "vpc_id" {
                "subnet_id"
            } else {
                "vpc_id"
            };
            if !super::vpc_gateways::absent(v, m, other)
                || !matches!(
                    known("internet_gateway_exclusion_mode"),
                    Some("allow-bidirectional" | "allow-egress")
                )
            {
                continue;
            }
        }
        if kind == "aws_vpc_encryption_control"
            && !matches!(known("mode"), Some("monitor" | "enforce"))
        {
            continue;
        }
        r.service_reference_verified = true;
    }
}

pub(super) fn association(raw: &crate::semantic::Input, target: usize) -> Option<(usize, usize)> {
    let get = |path: &str| {
        let mut found = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == path);
        let r = found.next()?;
        (found.next().is_none() && super::service_references::verified(raw, r))
            .then(|| r.sources[0])
    };
    Some((get("dhcp_options_id")?, get("vpc_id")?))
}
