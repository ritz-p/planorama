use crate::icons::ResourceIcon;

pub(super) fn icon_for(resource_type: &str) -> Option<ResourceIcon> {
    Some(match resource_type {
        "aws_organizations_policy" | "aws_organizations_policy_attachment" => {
            ResourceIcon::Organizations
        }
        "aws_organizations_organization"
        | "aws_organizations_organizational_unit"
        | "aws_organizations_account" => ResourceIcon::Organizations,
        "aws_ecr_repository" | "aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy" => {
            ResourceIcon::Ecr
        }
        "aws_wafv2_web_acl" | "aws_wafv2_rule_group" | "aws_wafv2_web_acl_association" => {
            ResourceIcon::Waf
        }
        "aws_acmpca_certificate_authority"
        | "aws_acmpca_certificate"
        | "aws_acmpca_certificate_authority_certificate" => ResourceIcon::PrivateCa,
        "aws_athena_workgroup" | "aws_athena_data_catalog" => ResourceIcon::Athena,
        "aws_glue_catalog_database"
        | "aws_glue_catalog_table"
        | "aws_glue_crawler"
        | "aws_glue_job" => ResourceIcon::Glue,
        "aws_efs_file_system" | "aws_efs_mount_target" | "aws_efs_access_point" => {
            ResourceIcon::Efs
        }
        "aws_ebs_volume" | "aws_volume_attachment" => ResourceIcon::Ebs,
        "aws_ssm_parameter" => ResourceIcon::Ssm,
        "aws_secretsmanager_secret" | "aws_secretsmanager_secret_version" => {
            ResourceIcon::SecretsManager
        }
        "aws_kms_key" | "aws_kms_alias" => ResourceIcon::Kms,
        "aws_s3_bucket_server_side_encryption_configuration" => ResourceIcon::Storage,
        "aws_acm_certificate" | "aws_acm_certificate_validation" => ResourceIcon::Acm,
        "aws_route53_zone" | "aws_route53_record" => ResourceIcon::Route53,
        "aws_vpc"
        | "aws_vpcs"
        | "aws_vpc_peering_connection"
        | "aws_vpc_peering_connections"
        | "aws_vpc_dhcp_options"
        | "aws_vpc_ipam"
        | "aws_vpc_ipams"
        | "aws_vpc_ipam_pool"
        | "aws_vpc_ipam_pools"
        | "aws_vpc_ipam_pool_cidrs"
        | "aws_prefix_list"
        | "aws_ec2_managed_prefix_list"
        | "aws_ec2_managed_prefix_lists"
        | "aws_ec2_network_insights_path"
        | "aws_network_interface"
        | "aws_network_interfaces" => ResourceIcon::Vpc,
        "aws_subnet" | "aws_subnets" => ResourceIcon::Subnet,
        "aws_instance"
        | "aws_instances"
        | "aws_ami"
        | "aws_ami_ids"
        | "aws_eip"
        | "aws_eips"
        | "aws_launch_template"
        | "aws_ec2_capacity_block_reservation"
        | "aws_ec2_host"
        | "aws_ec2_hosts"
        | "aws_key_pair"
        | "aws_ec2_public_ipv4_pool"
        | "aws_ec2_public_ipv4_pools"
        | "aws_spot_datafeed_subscription" => ResourceIcon::Ec2,
        "aws_internet_gateway"
        | "aws_nat_gateway"
        | "aws_nat_gateways"
        | "aws_customer_gateway"
        | "aws_vpn_gateway"
        | "aws_vpn_connection"
        | "aws_ec2_transit_gateway"
        | "aws_ec2_transit_gateway_attachment"
        | "aws_ec2_transit_gateway_attachments"
        | "aws_ec2_transit_gateway_connect"
        | "aws_ec2_transit_gateway_connect_peer"
        | "aws_ec2_transit_gateway_dx_gateway_attachment"
        | "aws_ec2_transit_gateway_multicast_domain"
        | "aws_ec2_transit_gateway_vpc_attachment"
        | "aws_ec2_transit_gateway_vpc_attachments"
        | "aws_ec2_transit_gateway_vpn_attachment"
        | "aws_ec2_transit_gateway_peering_attachment"
        | "aws_ec2_transit_gateway_peering_attachments" => ResourceIcon::Gateway,
        "aws_route_table"
        | "aws_route_tables"
        | "aws_route"
        | "aws_route_table_association"
        | "aws_ec2_transit_gateway_route_table"
        | "aws_ec2_transit_gateway_route_tables"
        | "aws_ec2_transit_gateway_route"
        | "aws_ec2_transit_gateway_route_table_association"
        | "aws_ec2_transit_gateway_route_table_associations"
        | "aws_ec2_transit_gateway_route_table_propagation"
        | "aws_ec2_transit_gateway_route_table_propagations"
        | "aws_ec2_transit_gateway_route_table_routes" => ResourceIcon::Route,
        "aws_vpc_endpoint"
        | "aws_vpc_endpoint_service"
        | "aws_vpc_endpoint_associations"
        | "aws_vpc_endpoint_route_table_association"
        | "aws_vpc_endpoint_subnet_association" => ResourceIcon::Endpoint,
        "aws_lb"
        | "aws_lbs"
        | "aws_alb"
        | "aws_elb"
        | "aws_lb_listener"
        | "aws_alb_listener"
        | "aws_lb_listener_rule"
        | "aws_lb_target_group"
        | "aws_alb_target_group"
        | "aws_lb_trust_store"
        | "aws_lb_target_group_attachment" => ResourceIcon::LoadBalancer,
        "aws_sqs_queue" => ResourceIcon::Sqs,
        "aws_kinesis_stream" => ResourceIcon::Kinesis,
        "aws_kinesis_firehose_delivery_stream" => ResourceIcon::Firehose,
        "aws_sns_topic" => ResourceIcon::Sns,
        "aws_cloudwatch_event_bus"
        | "aws_cloudwatch_event_rule"
        | "aws_cloudwatch_event_target" => ResourceIcon::EventBridge,
        "aws_lambda_function"
        | "aws_lambda_functions"
        | "aws_lambda_alias"
        | "aws_lambda_function_url"
        | "aws_lambda_layer_version"
        | "aws_lambda_code_signing_config" => ResourceIcon::Lambda,
        "aws_ecs_cluster"
        | "aws_ecs_clusters"
        | "aws_ecs_service"
        | "aws_ecs_task_definition"
        | "aws_ecs_container_definition" => ResourceIcon::Ecs,
        "aws_eks_cluster"
        | "aws_eks_clusters"
        | "aws_eks_node_group"
        | "aws_eks_node_groups"
        | "aws_eks_addon"
        | "aws_eks_fargate_profile"
        | "aws_eks_access_entry" => ResourceIcon::Eks,
        "aws_db_instance"
        | "aws_db_instances"
        | "aws_db_subnet_group"
        | "aws_db_snapshot"
        | "aws_db_cluster_snapshot"
        | "aws_db_parameter_group"
        | "aws_db_proxy"
        | "aws_db_proxy_default_target_group"
        | "aws_db_proxy_target"
        | "aws_rds_cluster"
        | "aws_rds_cluster_instance"
        | "aws_rds_clusters"
        | "aws_rds_cluster_parameter_group"
        | "aws_rds_global_cluster"
        | "aws_rds_snapshots" => ResourceIcon::Database,
        "aws_dynamodb_table"
        | "aws_dynamodb_tables"
        | "aws_dynamodb_backups"
        | "aws_dynamodb_table_item" => ResourceIcon::DynamoDb,
        "aws_elasticache_cluster"
        | "aws_elasticache_replication_group"
        | "aws_elasticache_subnet_group"
        | "aws_elasticache_serverless_cache"
        | "aws_elasticache_user" => ResourceIcon::Cache,
        "aws_opensearch_domain" | "aws_elasticsearch_domain" => ResourceIcon::Search,
        "aws_s3_bucket"
        | "aws_s3_buckets"
        | "aws_s3_object"
        | "aws_s3_objects"
        | "aws_s3_bucket_object"
        | "aws_s3_bucket_objects"
        | "aws_s3_bucket_notification"
        | "aws_s3_bucket_object_lock_configuration"
        | "aws_s3_bucket_replication_configuration"
        | "aws_s3_directory_buckets"
        | "aws_s3_bucket_policy"
        | "aws_s3_access_point"
        | "aws_s3control_access_points"
        | "aws_s3control_multi_region_access_point"
        | "aws_s3control_multi_region_access_points"
        | "aws_s3_account_public_access_block" => ResourceIcon::Storage,
        "aws_cloudfront_distribution"
        | "aws_cloudfront_function"
        | "aws_cloudfront_origin_access_identity"
        | "aws_cloudfront_origin_access_identities"
        | "aws_cloudfront_origin_access_control"
        | "aws_cloudfront_cache_policy"
        | "aws_cloudfront_origin_request_policy"
        | "aws_cloudfront_response_headers_policy"
        | "aws_cloudfront_connection_group"
        | "aws_cloudfront_distribution_tenant"
        | "aws_cloudfront_realtime_log_config" => ResourceIcon::CloudFront,
        "aws_api_gateway_rest_api"
        | "aws_api_gateway_stage"
        | "aws_api_gateway_domain_name"
        | "aws_api_gateway_resource"
        | "aws_api_gateway_vpc_link"
        | "aws_api_gateway_api_key"
        | "aws_api_gateway_api_keys"
        | "aws_api_gateway_authorizer"
        | "aws_api_gateway_authorizers"
        | "aws_apigatewayv2_api"
        | "aws_apigatewayv2_apis"
        | "aws_apigatewayv2_vpc_link" => ResourceIcon::ApiGateway,
        "aws_iam_role"
        | "aws_iam_roles"
        | "aws_iam_policy"
        | "aws_iam_role_policy"
        | "aws_iam_role_policies"
        | "aws_iam_role_policy_attachments"
        | "aws_iam_instance_profile"
        | "aws_iam_instance_profiles"
        | "aws_iam_user"
        | "aws_iam_user_policy"
        | "aws_iam_group_policy"
        | "aws_iam_group_membership"
        | "aws_iam_user_group_membership"
        | "aws_iam_user_policy_attachment"
        | "aws_iam_group_policy_attachment"
        | "aws_iam_users"
        | "aws_iam_group"
        | "aws_iam_openid_connect_provider"
        | "aws_iam_saml_provider"
        | "aws_iam_access_keys"
        | "aws_iam_server_certificate"
        | "aws_iam_user_ssh_key"
        | "aws_iam_role_policy_attachment" => ResourceIcon::Iam,
        "aws_security_group"
        | "aws_security_groups"
        | "aws_network_acl"
        | "aws_network_acls"
        | "aws_vpc_security_group_rule"
        | "aws_vpc_security_group_rules" => ResourceIcon::Security,
        "aws_cloudwatch_log_group"
        | "aws_cloudwatch_log_groups"
        | "aws_cloudwatch_metric_alarm" => ResourceIcon::Monitoring,
        "aws_autoscaling_group"
        | "aws_autoscaling_groups"
        | "aws_autoscaling_policy"
        | "aws_launch_configuration" => ResourceIcon::Scaling,
        _ => return None,
    })
}
