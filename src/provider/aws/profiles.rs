use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const VPC: &str = "aws_route53profiles_association";
const RESOURCE: &str = "aws_route53profiles_resource_association";

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        (VPC | RESOURCE, "profile_id") => Some("Route 53 profile"),
        (VPC, "resource_id") => Some("Profile VPC"),
        (RESOURCE, "resource_arn") => Some("Profile resource"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        let [s] = r.sources.as_slice() else { continue };
        let endpoint = &nodes[*s];
        let field = match (
            node.resource_type.as_str(),
            r.attribute.as_str(),
            endpoint.resource_type.as_str(),
        ) {
            (VPC | RESOURCE, "profile_id", "aws_route53profiles_profile")
            | (VPC, "resource_id", "aws_vpc") => "id",
            (
                RESOURCE,
                "resource_arn",
                "aws_route53_zone"
                | "aws_route53_resolver_rule"
                | "aws_route53_resolver_firewall_rule_group",
            ) => "arn",
            _ => continue,
        };
        if super::route53::endpoint(nodes, r, &endpoint.resource_type, &[field]).is_none()
            || !super::route53::configured(node, endpoint)
        {
            continue;
        }
        let actual = super::service_evidence::known(evidence, r.target, &r.attribute);
        if actual.is_none() || actual != super::service_evidence::known(evidence, *s, field) {
            continue;
        }
        let left = super::scope::scope(node).and_then(|s| s.region);
        let right = super::scope::scope(endpoint).and_then(|s| s.region);
        if left.is_none() || (endpoint.resource_type != "aws_route53_zone" && left != right) {
            continue;
        }
        if endpoint.resource_type == "aws_route53_zone" {
            let (v, m) = evidence[*s];
            if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_))
                || !matches!(&m["vpc"], Value::Null | Value::Bool(false))
                || v["vpc"].as_array().is_none_or(|a| a.is_empty())
            {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    super::route53::bindings(raw, r)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/profiles.rs"]
mod tests;
