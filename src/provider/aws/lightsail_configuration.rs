use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_lightsail_disk_attachment", "disk_name") => Rule::new(
            "Lightsail disk",
            &[("aws_lightsail_disk", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_static_ip_attachment", "static_ip_name") => Rule::new(
            "Static IP",
            &[("aws_lightsail_static_ip", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_attachment", "lb_name") => Rule::new(
            "Lightsail load balancer",
            &[("aws_lightsail_lb", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_bucket_resource_access", "bucket_name") => Rule::new(
            "Lightsail bucket",
            &[("aws_lightsail_bucket", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_bucket_access_key", "bucket_name") => Rule::new(
            "Lightsail bucket",
            &[("aws_lightsail_bucket", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_instance_public_ports", "instance_name") => Rule::new(
            "Lightsail instance",
            &[("aws_lightsail_instance", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_container_service_deployment_version", "service_name") => Rule::new(
            "Container service",
            &[("aws_lightsail_container_service", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_certificate", "lb_name") => Rule::new(
            "Lightsail load balancer",
            &[("aws_lightsail_lb", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_certificate_attachment", "lb_name") => Rule::new(
            "Lightsail load balancer",
            &[("aws_lightsail_lb", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_https_redirection_policy", "lb_name") => Rule::new(
            "Lightsail load balancer",
            &[("aws_lightsail_lb", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_stickiness_policy", "lb_name") => Rule::new(
            "Lightsail load balancer",
            &[("aws_lightsail_lb", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_domain_entry", "domain_name") => Rule::new(
            "Lightsail domain",
            &[("aws_lightsail_domain", &["domain_name"])],
            false,
            &[],
        ),
        (
            "aws_lightsail_disk_attachment"
            | "aws_lightsail_static_ip_attachment"
            | "aws_lightsail_lb_attachment",
            "instance_name",
        ) => Rule::new(
            "Attached Lightsail instance",
            &[("aws_lightsail_instance", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_bucket_resource_access", "resource_name") => Rule::new(
            "Lightsail instance with bucket access",
            &[("aws_lightsail_instance", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_lb_certificate_attachment", "certificate_name") => Rule::new(
            "Load balancer certificate",
            &[("aws_lightsail_lb_certificate", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_distribution", "origin.name") => Rule::new(
            "Distribution origin",
            &[
                ("aws_lightsail_instance", &["name"]),
                ("aws_lightsail_lb", &["name"]),
                ("aws_lightsail_container_service", &["name"]),
                ("aws_lightsail_bucket", &["name"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_lightsail_distribution", "certificate_name") => Rule::new(
            "Distribution certificate",
            &[("aws_lightsail_certificate", &["name"])],
            false,
            &[],
        ),
        ("aws_lightsail_container_service", "public_domain_names.certificate.certificate_name") => {
            Rule::new(
                "Container service certificate",
                &[("aws_lightsail_certificate", &["name"])],
                false,
                &["public_domain_names.certificate"],
            )
        }
        ("aws_lightsail_instance", "key_pair_name") => Rule::new(
            "Lightsail key pair",
            &[("aws_lightsail_key_pair", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_lightsail_disk_attachment"
            | "aws_lightsail_static_ip_attachment"
            | "aws_lightsail_lb_attachment"
            | "aws_lightsail_bucket_resource_access"
            | "aws_lightsail_bucket_access_key"
            | "aws_lightsail_instance_public_ports"
            | "aws_lightsail_container_service_deployment_version"
            | "aws_lightsail_lb_certificate"
            | "aws_lightsail_lb_certificate_attachment"
            | "aws_lightsail_lb_https_redirection_policy"
            | "aws_lightsail_lb_stickiness_policy"
            | "aws_lightsail_domain_entry"
            | "aws_lightsail_distribution"
            | "aws_lightsail_container_service"
            | "aws_lightsail_instance"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
        && (raw.nodes[r.target].resource_type != "aws_lightsail_distribution"
            || r.attribute != "origin.name"
            || configured_links::destination_in_same_account(raw, r))
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    let consumer = &e.nodes[r.target];
    let endpoint = &e.nodes[source];
    let region = |node: &TerraformEntity| super::scope::scope(node).and_then(|s| s.region);
    match (consumer.resource_type.as_str(), r.attribute.as_str()) {
        ("aws_lightsail_distribution", "origin.name") => {
            if region(consumer).is_none_or(|region| region.0 != "us-east-1")
                || region(endpoint).is_none_or(|region| {
                    e.text(r.target, "origin.0.region_name") != Some(region.0.as_str())
                })
            {
                return false;
            }
            let kind = match endpoint.resource_type.as_str() {
                "aws_lightsail_instance" => "Instance",
                "aws_lightsail_lb" => "LoadBalancer",
                "aws_lightsail_bucket" => "Bucket",
                "aws_lightsail_container_service" => "ContainerService",
                _ => return false,
            };
            e.text(r.target, "origin.0.resource_type")
                .is_none_or(|actual| actual == kind)
        }
        ("aws_lightsail_distribution", "certificate_name")
        | ("aws_lightsail_domain_entry", "domain_name") => {
            region(consumer).is_some_and(|region| region.0 == "us-east-1")
        }
        ("aws_lightsail_lb_certificate_attachment", "certificate_name") => e.same_endpoint(
            r.target,
            "lb_name",
            source,
            "lb_name",
            "aws_lightsail_lb",
            "name",
        ),
        _ => true,
    }
}
pub(super) fn owner_member(kind: &str) -> bool {
    matches!(
        kind,
        "aws_lightsail_disk_attachment"
            | "aws_lightsail_static_ip_attachment"
            | "aws_lightsail_lb_attachment"
            | "aws_lightsail_bucket_resource_access"
            | "aws_lightsail_bucket_access_key"
            | "aws_lightsail_instance_public_ports"
            | "aws_lightsail_container_service_deployment_version"
            | "aws_lightsail_lb_certificate"
            | "aws_lightsail_lb_certificate_attachment"
            | "aws_lightsail_lb_https_redirection_policy"
            | "aws_lightsail_lb_stickiness_policy"
            | "aws_lightsail_domain_entry"
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lightsail_configuration.rs"]
mod tests;
