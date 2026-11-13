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
            kind if super::directory_service::owner(kind).is_some() => {
                super::directory_service::owner(kind).unwrap()
            }
            kind if super::transfer_identity::owner(kind).is_some() => {
                super::transfer_identity::owner(kind).unwrap()
            }
            "aws_transfer_web_app_customization" => {
                ("web_app_id", "Web app owner", "Customization")
            }
            kind if super::redshift_snapshots::owner(kind).is_some() => {
                super::redshift_snapshots::owner(kind).unwrap()
            }
            "aws_redshift_endpoint_authorization" => (
                "cluster_identifier",
                "Authorized cluster",
                "Endpoint authorizations",
            ),
            "aws_redshift_cluster_iam_roles" => {
                ("cluster_identifier", "IAM roles cluster", "IAM roles")
            }
            "aws_transfer_agreement" => ("server_id", "Agreement server", "Agreements"),
            "aws_acmpca_permission" => {
                ("certificate_authority_arn", "Permission CA", "Permissions")
            }
            "aws_acmpca_policy" => ("resource_arn", "Policy CA", "Policies"),
            "aws_glue_trigger" => ("workflow_name", "Trigger workflow", "Triggers"),
            kind if super::glue_catalog::owner(kind).is_some() => {
                if kind == "aws_glue_schema"
                    && raw
                        .attributes
                        .iter()
                        .any(|r| r.target != index && r.sources.contains(&index))
                {
                    continue;
                }
                super::glue_catalog::owner(kind).unwrap()
            }
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
            "aws_secretsmanager_secret_policy" => {
                ("secret_arn", "Secret owner", "Secret configuration")
            }
            "aws_secretsmanager_secret_rotation" | "aws_secretsmanager_tag" => {
                ("secret_id", "Secret owner", "Secret configuration")
            }
            "aws_s3files_access_point"
            | "aws_s3files_file_system_policy"
            | "aws_s3files_synchronization_configuration" => (
                "file_system_id",
                "S3 Files file system",
                "File system configuration",
            ),
            "aws_s3tables_namespace"
            | "aws_s3tables_table_bucket_policy"
            | "aws_s3tables_table_bucket_replication" => (
                "table_bucket_arn",
                "Table bucket",
                "Table bucket configuration",
            ),
            "aws_s3tables_table_policy" => ("name", "Qualified table owner", "Table configuration"),
            "aws_s3tables_table_replication" => (
                "table_arn",
                "Replication source table",
                "Table configuration",
            ),
            "aws_route53_hosted_zone_dnssec" | "aws_route53_key_signing_key" => (
                "hosted_zone_id",
                "DNSSEC hosted zone",
                "DNSSEC configuration",
            ),
            "aws_route53domains_delegation_signer_record" => (
                "domain_name",
                "Registered domain owner",
                "Delegation signers",
            ),
            kind if kind != "aws_elb_attachment"
                && super::classic_elb::owner_path(kind).is_some() =>
            {
                (
                    super::classic_elb::owner_path(kind).unwrap(),
                    "Classic load balancer",
                    "Classic ELB policies",
                )
            }
            "aws_oam_sink_policy" => ("sink_identifier", "OAM policy sink", "Sink configuration"),
            "aws_rds_shard_group" => (
                "db_cluster_identifier",
                "Shard group cluster",
                "Shard groups",
            ),
            "aws_rum_metrics_destination" => (
                "app_monitor_name",
                "RUM app monitor",
                "Metrics destinations",
            ),
            "aws_vpclattice_resource_configuration" => (
                "resource_configuration_group_id",
                "Parent resource group",
                "Child resources",
            ),
            "aws_vpclattice_listener" => {
                let path = if raw
                    .attributes
                    .iter()
                    .any(|r| r.target == index && r.attribute == "service_identifier")
                {
                    "service_identifier"
                } else {
                    "service_arn"
                };
                (path, "Lattice service", "Listeners")
            }
            "aws_vpclattice_listener_rule" => {
                ("listener_identifier", "Rule listener", "Listener rules")
            }
            "aws_s3vectors_index" => (
                "vector_bucket_name",
                "Vector bucket owner",
                "Vector indexes",
            ),
            "aws_s3vectors_vector_bucket_policy" => (
                "vector_bucket_arn",
                "Vector bucket owner",
                "Vector bucket configuration",
            ),
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
            "aws_networkmonitor_probe" => ("monitor_name", "Probe monitor", "Probes"),
            "aws_networkfirewall_logging_configuration" => {
                ("firewall_arn", "Logging firewall", "Logging")
            }
            "aws_ecs_task_set" => ("service", "Task set service", "Task sets"),
            "aws_kms_alias" => ("target_key_id", "Alias key", "Aliases"),
            "aws_kms_grant" | "aws_kms_key_policy" => {
                ("key_id", "Configured KMS key", "Key configuration")
            }
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
