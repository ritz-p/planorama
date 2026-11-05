use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/default_network.rs"]
mod tests;

pub(super) fn parent_field(kind: &str) -> Option<&'static str> {
    match kind {
        "aws_default_network_acl" => Some("default_network_acl_id"),
        "aws_default_route_table" => Some("default_route_table_id"),
        "aws_default_security_group" => Some("vpc_id"),
        _ => None,
    }
}

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    (parent_field(kind) == Some(field)).then_some("Adopted VPC context")
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none()
            && !(nodes[r.target].resource_type == "aws_route"
                && r.attribute == "route_table_id"
                && r.sources
                    .iter()
                    .any(|&s| nodes[s].resource_type == "aws_default_route_table"))
        {
            continue;
        }
        r.service_reference_verified = false;
        let [source] = r.sources.as_slice() else {
            continue;
        };
        if nodes[r.target].resource_type == "aws_route" {
            r.service_reference_verified = super::vpc_routes::eligible(nodes, r)
                && matches!(
                    nodes[*source].resource_type.as_str(),
                    "aws_route_table" | "aws_default_route_table"
                )
                && r.ids_only
                && r.scalar_id_matches;
            continue;
        }
        let output = if r.attribute == "vpc_id" {
            "id"
        } else {
            &r.attribute
        };
        if !super::vpc_routes::eligible(nodes, r)
            || !matches!(
                nodes[*source].resource_type.as_str(),
                "aws_vpc" | "aws_default_vpc"
            )
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(output)
        {
            continue;
        }
        let known = |i, key| super::service_evidence::known(evidence, i, key);
        let actual = known(r.target, &r.attribute);
        if actual.is_none() || actual != known(*source, output) {
            continue;
        }
        if r.attribute != "vpc_id"
            && (known(r.target, "id") != actual
                || known(*source, "id").is_none()
                || known(r.target, "vpc_id") != known(*source, "id"))
        {
            continue;
        }
        r.service_reference_verified = true;
    }
}
