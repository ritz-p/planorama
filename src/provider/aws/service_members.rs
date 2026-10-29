use crate::model::{Graph, PresentationOwner};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/kms.rs"]
mod kms_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/secrets_manager.rs"]
mod secrets_manager_tests;

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() {
            continue;
        }
        let (attribute, label, section) = match node.resource_type.as_str() {
            "aws_security_group_rule"
            | "aws_vpc_security_group_ingress_rule"
            | "aws_vpc_security_group_egress_rule" => {
                ("security_group_id", "Security group owner", "Rules")
            }
            "aws_vpc_security_group_rules_exclusive" => (
                "security_group_id",
                "Security group owner",
                "Rule management",
            ),
            "aws_vpc_endpoint_policy" => ("vpc_endpoint_id", "Configured endpoint", "Policies"),
            "aws_vpc_endpoint_private_dns" => {
                ("vpc_endpoint_id", "Configured endpoint", "Private DNS")
            }
            "aws_vpc_endpoint_service_allowed_principal" => (
                "vpc_endpoint_service_id",
                "Configured service",
                "Allowed principals",
            ),
            "aws_vpc_endpoint_service_private_dns_verification" => {
                ("service_id", "Configured service", "DNS verification")
            }
            "aws_vpc_endpoint_connection_notification" => {
                let field = if raw.attributes.iter().any(|r| {
                    r.target == index
                        && r.attribute == "vpc_endpoint_id"
                        && super::service_references::verified(raw, r)
                }) {
                    "vpc_endpoint_id"
                } else {
                    "vpc_endpoint_service_id"
                };
                (field, "Notification owner", "Notifications")
            }
            "aws_eip_domain_name" => ("allocation_id", "Elastic IP allocation", "Reverse DNS"),
            "aws_networkfirewall_logging_configuration" => {
                ("firewall_arn", "Logging firewall", "Logging")
            }
            "aws_kms_alias" => ("target_key_id", "Alias key", "Aliases"),
            "aws_secretsmanager_secret_version" => ("secret_id", "Version secret", "Versions"),
            "aws_glue_catalog_table" => {
                if raw.attributes.iter().any(|reference| {
                    reference.target != index && reference.sources.contains(&index)
                }) {
                    continue;
                }
                ("database_name", "Catalog database", "Tables")
            }
            "aws_acm_certificate" => {
                if raw.attributes.iter().any(|reference| {
                    reference.target != index && reference.sources.contains(&index)
                }) {
                    continue;
                }
                (
                    "certificate_authority_arn",
                    "Signing CA reference",
                    "Certificates",
                )
            }
            _ => continue,
        };
        let mut candidates = raw
            .attributes
            .iter()
            .filter(|reference| reference.target == index && reference.attribute == attribute);
        let Some(reference) = candidates.next() else {
            continue;
        };
        if candidates.next().is_some() || !super::service_references::verified(raw, reference) {
            continue;
        }
        let [owner] = reference.sources.as_slice() else {
            continue;
        };
        if !node.presentation_scope_matches(&raw.nodes[*owner]) {
            continue;
        }
        graph.nodes[index].member_owner = Some(PresentationOwner {
            target: raw.nodes[*owner].entity.provenance[0].clone(),
            attribute,
            label,
            section,
        });
        if node.resource_type == "aws_networkfirewall_logging_configuration" {
            graph.nodes[index].placement = crate::model::Placement::Spatial;
        }
    }
}
