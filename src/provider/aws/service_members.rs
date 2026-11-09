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
            kind if super::identity_center::owner(kind).is_some() => {
                let (field, label) = super::identity_center::owner(kind).unwrap();
                (field, label, "Identity Center configuration")
            }
            kind if super::iam_credentials::owner(kind).is_some() => (
                super::iam_credentials::owner(kind).unwrap(),
                "Credential owner",
                "Credentials",
            ),
            kind if super::iam_policies::owner(kind).is_some() => {
                let (field, _) = super::iam_policies::owner(kind).unwrap();
                (
                    field,
                    super::iam_policies::label(kind, field).unwrap(),
                    "Policy management",
                )
            }
            "aws_globalaccelerator_listener" | "aws_globalaccelerator_custom_routing_listener" => {
                ("accelerator_arn", "Listener accelerator", "Listeners")
            }
            "aws_globalaccelerator_endpoint_group"
            | "aws_globalaccelerator_custom_routing_endpoint_group" => {
                ("listener_arn", "Group listener", "Endpoint groups")
            }
            "aws_ec2_traffic_mirror_filter_rule" => (
                "traffic_mirror_filter_id",
                "Traffic mirror filter",
                "Mirror rules",
            ),
            "aws_vpc_route_server_peer" => {
                ("route_server_endpoint_id", "BGP peer endpoint", "BGP peers")
            }
            "aws_ec2_subnet_cidr_reservation" => {
                ("subnet_id", "Reservation subnet", "CIDR reservations")
            }
            "aws_vpc_ipam_scope" => ("ipam_id", "Scope IPAM", "IPAM scopes"),
            "aws_vpc_ipam_pool_cidr"
            | "aws_vpc_ipam_pool_cidr_allocation"
            | "aws_vpc_ipam_preview_next_cidr" => {
                ("ipam_pool_id", "IPAM pool owner", "Pool configuration")
            }
            "aws_vpc_ipv4_cidr_block_association" | "aws_vpc_ipv6_cidr_block_association" => {
                ("vpc_id", "CIDR VPC owner", "CIDR associations")
            }
            "aws_ec2_managed_prefix_list_entry" => {
                ("prefix_list_id", "Prefix list owner", "Prefix list entries")
            }
            "aws_vpc_encryption_control" => {
                ("vpc_id", "Encryption control VPC", "Encryption control")
            }
            "aws_vpc_block_public_access_exclusion" => {
                let field = if raw.attributes.iter().any(|r| {
                    r.target == index
                        && r.attribute == "vpc_id"
                        && super::service_references::verified(raw, r)
                }) {
                    "vpc_id"
                } else {
                    "subnet_id"
                };
                (field, "Exclusion target", "Public access exclusions")
            }
            "aws_cloudfront_monitoring_subscription" => {
                ("distribution_id", "Monitoring distribution", "Monitoring")
            }
            kind if super::cloudfront_functions::helper(kind) => (
                "key_value_store_arn",
                "Owner KeyValueStore",
                "Key management",
            ),
            kind if super::search::helper(kind) => {
                ("domain_name", "Search domain", "Domain configuration")
            }
            "aws_efs_backup_policy" | "aws_efs_file_system_policy" => {
                ("file_system_id", "File system", "Policies")
            }
            kind if super::backup_vaults::helper(kind) => (
                "backup_vault_name",
                "Backup vault owner",
                "Vault configuration",
            ),
            "aws_flow_log" => {
                let Some(field) = super::flow_logs::MONITORED.iter().copied().find(|field| {
                    raw.attributes.iter().any(|r| {
                        r.target == index
                            && r.attribute == *field
                            && super::service_references::verified(raw, r)
                    })
                }) else {
                    continue;
                };
                (field, "Monitored resource", "Flow logs")
            }
            "aws_network_acl_rule" => ("network_acl_id", "Network ACL owner", "Rules"),
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
            "aws_network_interface_permission" => {
                ("network_interface_id", "Network interface", "Permissions")
            }
            "aws_vpc_peering_connection_accepter" => (
                "vpc_peering_connection_id",
                "Peering connection",
                "Acceptance",
            ),
            "aws_vpc_peering_connection_options" => {
                ("vpc_peering_connection_id", "Peering connection", "Options")
            }
            "aws_route53_records_exclusive" => ("zone_id", "Hosted zone", "Record management"),
            "aws_appsync_channel_namespace" => {
                ("api_id", "Namespace Event API", "Channel namespaces")
            }
            "aws_appsync_datasource" => ("api_id", "Data source GraphQL API", "Data sources"),
            "aws_appsync_api_cache" => ("api_id", "Cache GraphQL API", "API cache"),
            "aws_appsync_api_key" => ("api_id", "AppSync key API", "API keys"),
            "aws_appsync_type" => ("api_id", "Type GraphQL API", "Schema types"),
            "aws_s3control_access_point_policy" => (
                "access_point_arn",
                "Access point policy owner",
                "Access point policy",
            ),
            "aws_s3control_bucket_policy" => ("bucket", "Outposts bucket owner", "Bucket policy"),
            "aws_s3control_bucket_lifecycle_configuration" => {
                ("bucket", "Outposts bucket owner", "Bucket lifecycle")
            }
            "aws_s3control_object_lambda_access_point_policy" => {
                ("name", "Named access point owner", "Object Lambda policy")
            }
            "aws_route53_resolver_firewall_rule" => (
                "firewall_rule_group_id",
                "DNS Firewall rule group",
                "DNS Firewall rules",
            ),
            "aws_route53_query_log" => ("zone_id", "Query logging hosted zone", "Query logging"),
            "aws_route53_resolver_config"
            | "aws_route53_resolver_dnssec_config"
            | "aws_route53_resolver_firewall_config" => (
                "resource_id",
                "Resolver VPC owner",
                "Resolver configuration",
            ),
            "aws_eip_domain_name" => ("allocation_id", "Elastic IP allocation", "Reverse DNS"),
            "aws_networkfirewall_logging_configuration" => {
                ("firewall_arn", "Logging firewall", "Logging")
            }
            "aws_ecs_task_set" => ("service", "Task set service", "Task sets"),
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
        if ((matches!(
            node.resource_type.as_str(),
            "aws_globalaccelerator_listener"
                | "aws_globalaccelerator_custom_routing_listener"
                | "aws_vpc_route_server_peer"
                | "aws_flow_log"
                | "aws_cloudfront_monitoring_subscription"
        ) || super::ipam::owner(&node.resource_type).is_some())
            && raw.nodes[*owner].placement == crate::model::Placement::Spatial)
            || node.resource_type == "aws_appsync_channel_namespace"
            || super::search::helper(&node.resource_type)
            || matches!(
                node.resource_type.as_str(),
                "aws_networkfirewall_logging_configuration"
                    | "aws_efs_backup_policy"
                    | "aws_efs_file_system_policy"
                    | "aws_backup_vault_lock_configuration"
                    | "aws_backup_vault_notifications"
                    | "aws_backup_vault_policy"
                    | "aws_vpc_encryption_control"
                    | "aws_vpc_block_public_access_exclusion"
                    | "aws_route53_records_exclusive"
                    | "aws_ecs_task_set"
            )
        {
            graph.nodes[index].placement = crate::model::Placement::Spatial;
        }
    }
}
