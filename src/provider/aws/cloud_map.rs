use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloud_map.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_service_discovery_private_dns_namespace", "vpc") => Some("Private DNS VPC context"),
        ("aws_service_discovery_service", "namespace_id" | "dns_config.namespace_id") => {
            Some("Cloud Map namespace")
        }
        ("aws_service_discovery_instance", "service_id") => Some("Cloud Map service"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        let [source] = r.sources.as_slice() else {
            continue;
        };
        if !super::vpc_routes::eligible(nodes, r) || !r.ids_only {
            continue;
        }
        let endpoint = nodes[*source].resource_type.as_str();
        let valid = match kind {
            "aws_service_discovery_private_dns_namespace" => {
                matches!(endpoint, "aws_vpc" | "aws_default_vpc")
            }
            "aws_service_discovery_instance" => endpoint == "aws_service_discovery_service",
            _ => matches!(
                endpoint,
                "aws_service_discovery_http_namespace"
                    | "aws_service_discovery_private_dns_namespace"
                    | "aws_service_discovery_public_dns_namespace"
            ),
        };
        let (value, unknown) = evidence[r.target];
        let actual = super::streaming::values(value, unknown, &r.attribute, false);
        let id = super::service_evidence::known(evidence, *source, "id");
        if !valid
            || id.is_none()
            || !actual.is_some_and(|s| s.len() == 1 && s.first().copied() == id)
        {
            continue;
        }
        if kind == "aws_service_discovery_service" {
            if !matches!(&unknown["dns_config"], Value::Null | Value::Bool(false))
                || !matches!(&unknown["namespace_id"], Value::Null | Value::Bool(false))
            {
                continue;
            }
            let dns = value["dns_config"].as_array();
            let has_dns = dns.is_some_and(|rows| rows.len() == 1);
            if !(value["dns_config"].is_null() || dns.is_some_and(|rows| rows.len() <= 1)) {
                continue;
            }
            let service_type = super::service_evidence::known(evidence, r.target, "type");
            if has_dns {
                if endpoint == "aws_service_discovery_http_namespace"
                    || !matches!(service_type, Some("DNS" | "DNS_HTTP"))
                    || dns.unwrap()[0]["namespace_id"].as_str() != id
                {
                    continue;
                }
            } else if service_type != Some("HTTP") {
                continue;
            }
            if value["namespace_id"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && Some(s) != id)
            {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
}
