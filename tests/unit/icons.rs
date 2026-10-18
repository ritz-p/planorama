use super::*;

#[test]
fn provider_lookup_shares_service_icons_and_handles_unknown_types() {
    for (resource_type, expected) in [
        ("aws_vpc", ResourceIcon::Vpc),
        ("aws_subnet", ResourceIcon::Subnet),
        ("aws_instance", ResourceIcon::Ec2),
        ("aws_ami", ResourceIcon::Ec2),
        ("aws_lb", ResourceIcon::LoadBalancer),
        ("aws_lb_listener", ResourceIcon::LoadBalancer),
        ("aws_lambda_function", ResourceIcon::Lambda),
        ("aws_sqs_queue", ResourceIcon::Sqs),
        ("aws_kinesis_stream", ResourceIcon::Kinesis),
        (
            "aws_kinesis_firehose_delivery_stream",
            ResourceIcon::Firehose,
        ),
        ("aws_sns_topic", ResourceIcon::Sns),
        ("aws_ecs_service", ResourceIcon::Ecs),
        ("aws_eks_cluster", ResourceIcon::Eks),
        ("aws_db_instance", ResourceIcon::Database),
        ("aws_dynamodb_table", ResourceIcon::DynamoDb),
        ("aws_elasticache_cluster", ResourceIcon::Cache),
        ("aws_opensearch_domain", ResourceIcon::Search),
        ("aws_s3_bucket", ResourceIcon::Storage),
        ("aws_cloudfront_distribution", ResourceIcon::CloudFront),
        ("aws_apigatewayv2_api", ResourceIcon::ApiGateway),
        ("aws_iam_role", ResourceIcon::Iam),
        ("aws_security_group", ResourceIcon::Security),
        ("aws_cloudwatch_log_group", ResourceIcon::Monitoring),
        ("aws_autoscaling_group", ResourceIcon::Scaling),
        ("aws_autoscaling_policy", ResourceIcon::Scaling),
        ("aws_nat_gateway", ResourceIcon::Gateway),
        ("aws_route", ResourceIcon::Route),
        ("aws_vpc_endpoint", ResourceIcon::Endpoint),
    ] {
        assert_eq!(icon_for(Provider::Aws, resource_type), Some(expected));
        assert_eq!(icon_for(Provider::Other, resource_type), None);
    }
    for resource_type in [
        "",
        "aws_future_service",
        "aws_instance_fake",
        "aws_region",
        "aws_iam_policy_document",
        "aws_iam_session_context",
        "aws_ec2_instance_type",
        "aws_ecs_task_execution",
        "aws_eks_cluster_auth",
        "aws_lambda_invocation",
        "aws_lb_hosted_zone_id",
        "aws_vpc_ipam_preview_next_cidr",
        "aws_cloudwatch_contributor_managed_insight_rules",
        "aws_cloudwatch_log_data_protection_policy_document",
        "AWS_VPC",
        "azurerm_virtual_network",
    ] {
        assert_eq!(
            icon_for(
                Provider::from_identity(&crate::model::ProviderIdentity::inferred(resource_type)),
                resource_type
            ),
            None
        );
    }
}

#[test]
fn visible_data_source_types_share_their_managed_service_icon() {
    use crate::model::ProviderIdentity;

    for (data_type, managed_type) in [
        ("aws_vpcs", "aws_vpc"),
        ("aws_vpc_peering_connections", "aws_vpc_peering_connection"),
        ("aws_vpc_ipam_pool_cidrs", "aws_vpc"),
        ("aws_network_interfaces", "aws_vpc"),
        ("aws_subnets", "aws_subnet"),
        ("aws_instances", "aws_instance"),
        ("aws_ami_ids", "aws_ami"),
        ("aws_eips", "aws_eip"),
        ("aws_launch_template", "aws_instance"),
        ("aws_ec2_host", "aws_instance"),
        ("aws_nat_gateways", "aws_nat_gateway"),
        (
            "aws_ec2_transit_gateway_attachments",
            "aws_ec2_transit_gateway",
        ),
        ("aws_route_tables", "aws_route_table"),
        (
            "aws_ec2_transit_gateway_route_tables",
            "aws_ec2_transit_gateway_route_table",
        ),
        (
            "aws_ec2_transit_gateway_route_table_routes",
            "aws_ec2_transit_gateway_route",
        ),
        ("aws_vpc_endpoint_service", "aws_vpc_endpoint"),
        ("aws_lbs", "aws_lb"),
        ("aws_alb_listener", "aws_lb_listener"),
        ("aws_lb_trust_store", "aws_lb"),
        ("aws_lambda_alias", "aws_lambda_function"),
        ("aws_ecs_clusters", "aws_ecs_cluster"),
        ("aws_ecs_container_definition", "aws_ecs_task_definition"),
        ("aws_eks_clusters", "aws_eks_cluster"),
        ("aws_eks_access_entry", "aws_eks_cluster"),
        ("aws_db_snapshot", "aws_db_instance"),
        ("aws_db_proxy", "aws_db_instance"),
        ("aws_db_cluster_snapshot", "aws_rds_cluster"),
        ("aws_dynamodb_tables", "aws_dynamodb_table"),
        (
            "aws_elasticache_replication_group",
            "aws_elasticache_cluster",
        ),
        ("aws_elasticsearch_domain", "aws_opensearch_domain"),
        ("aws_s3_objects", "aws_s3_bucket"),
        ("aws_s3_bucket_objects", "aws_s3_bucket"),
        ("aws_s3_access_point", "aws_s3_bucket"),
        ("aws_cloudfront_cache_policy", "aws_cloudfront_distribution"),
        ("aws_apigatewayv2_apis", "aws_apigatewayv2_api"),
        ("aws_api_gateway_vpc_link", "aws_api_gateway_rest_api"),
        ("aws_api_gateway_api_key", "aws_api_gateway_rest_api"),
        ("aws_iam_roles", "aws_iam_role"),
        ("aws_iam_instance_profiles", "aws_iam_instance_profile"),
        ("aws_iam_openid_connect_provider", "aws_iam_role"),
        ("aws_iam_user_ssh_key", "aws_iam_role"),
        ("aws_security_groups", "aws_security_group"),
        ("aws_network_acls", "aws_network_acl"),
        ("aws_cloudwatch_log_groups", "aws_cloudwatch_log_group"),
        ("aws_autoscaling_groups", "aws_autoscaling_group"),
        ("aws_launch_configuration", "aws_autoscaling_group"),
    ] {
        let expected = icon_for(Provider::Aws, managed_type).unwrap();
        for provider in [
            ProviderIdentity::from_source("registry.terraform.io/hashicorp/aws"),
            ProviderIdentity::inferred(data_type),
        ] {
            assert_eq!(
                icon_for(Provider::from_identity(&provider), data_type),
                Some(expected),
                "{data_type} shares {managed_type}"
            );
        }
        for source in [
            "registry.terraform.io/acme/aws",
            "example.com/hashicorp/aws",
            "terraform.io/builtin/terraform",
        ] {
            let provider = ProviderIdentity::from_source(source);
            assert_eq!(
                icon_for(Provider::from_identity(&provider), data_type),
                None,
                "{source}: {data_type}"
            );
        }
        for unsupported in [format!("data.{data_type}"), format!("{data_type}_future")] {
            assert_eq!(icon_for(Provider::Aws, &unsupported), None);
        }
    }
}

#[test]
fn terraform_icons_require_the_builtin_source_and_exact_resource_types() {
    use crate::model::ProviderIdentity;

    for resource_type in ["terraform_data", "terraform_remote_state"] {
        for provider in [
            ProviderIdentity::from_source("terraform.io/builtin/terraform"),
            ProviderIdentity::from_source(" TERRAFORM.IO/BUILTIN/TERRAFORM "),
            ProviderIdentity::inferred(resource_type),
        ] {
            assert!(provider.is_terraform());
            assert!(!provider.is_aws());
            assert_eq!(
                icon_for(Provider::from_identity(&provider), resource_type),
                Some(ResourceIcon::Terraform)
            );
        }
        for source in [
            "",
            "terraform",
            "hashicorp/terraform",
            "registry.terraform.io/hashicorp/terraform",
            "builtin/terraform",
            "registry.terraform.io/builtin/terraform",
            "example.com/builtin/terraform",
            "terraform.io/acme/terraform",
            "terraform.io/builtin/other",
            "acme/custom",
            "hashicorp/aws",
        ] {
            let provider = ProviderIdentity::from_source(source);
            assert!(!provider.is_terraform(), "{source}");
            assert_eq!(
                icon_for(Provider::from_identity(&provider), resource_type),
                None,
                "{source}: {resource_type}"
            );
        }
        assert_eq!(icon_for(Provider::Other, resource_type), None);
    }
    for resource_type in [
        "",
        "terraform_future_resource",
        "terraform_data_extra",
        "terraform_remote_state_extra",
        "TERRAFORM_DATA",
        "data.terraform_remote_state",
        "null_resource",
        "aws_instance",
    ] {
        assert_eq!(icon_for(Provider::Terraform, resource_type), None);
        assert!(!ProviderIdentity::inferred(resource_type).is_terraform());
    }
}
