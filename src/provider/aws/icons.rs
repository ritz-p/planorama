use crate::icons::ResourceIcon;

pub(super) fn icon_for(resource_type: &str) -> Option<ResourceIcon> {
    Some(match resource_type {
        "aws_vpc" | "aws_vpcs" | "aws_vpc_peering_connection" | "aws_vpc_peering_connections" => {
            ResourceIcon::Vpc
        }
        "aws_subnet" | "aws_subnets" => ResourceIcon::Subnet,
        "aws_instance"
        | "aws_instances"
        | "aws_ami"
        | "aws_ami_ids"
        | "aws_eip"
        | "aws_eips"
        | "aws_launch_template" => ResourceIcon::Ec2,
        "aws_internet_gateway"
        | "aws_nat_gateway"
        | "aws_nat_gateways"
        | "aws_ec2_transit_gateway"
        | "aws_ec2_transit_gateway_vpc_attachment"
        | "aws_ec2_transit_gateway_peering_attachment" => ResourceIcon::Gateway,
        "aws_route_table"
        | "aws_route_tables"
        | "aws_route"
        | "aws_route_table_association"
        | "aws_ec2_transit_gateway_route_table"
        | "aws_ec2_transit_gateway_route_tables"
        | "aws_ec2_transit_gateway_route"
        | "aws_ec2_transit_gateway_route_table_association"
        | "aws_ec2_transit_gateway_route_table_propagation" => ResourceIcon::Route,
        "aws_vpc_endpoint"
        | "aws_vpc_endpoint_service"
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
        "aws_lambda_function"
        | "aws_lambda_functions"
        | "aws_lambda_alias"
        | "aws_lambda_function_url"
        | "aws_lambda_layer_version" => ResourceIcon::Lambda,
        "aws_ecs_cluster" | "aws_ecs_clusters" | "aws_ecs_service" | "aws_ecs_task_definition" => {
            ResourceIcon::Ecs
        }
        "aws_eks_cluster"
        | "aws_eks_clusters"
        | "aws_eks_node_group"
        | "aws_eks_node_groups"
        | "aws_eks_addon" => ResourceIcon::Eks,
        "aws_db_instance"
        | "aws_db_instances"
        | "aws_db_subnet_group"
        | "aws_db_snapshot"
        | "aws_db_cluster_snapshot"
        | "aws_rds_cluster"
        | "aws_rds_clusters" => ResourceIcon::Database,
        "aws_dynamodb_table" | "aws_dynamodb_tables" => ResourceIcon::DynamoDb,
        "aws_elasticache_cluster"
        | "aws_elasticache_replication_group"
        | "aws_elasticache_subnet_group" => ResourceIcon::Cache,
        "aws_opensearch_domain" => ResourceIcon::Search,
        "aws_s3_bucket"
        | "aws_s3_buckets"
        | "aws_s3_object"
        | "aws_s3_objects"
        | "aws_s3_bucket_object"
        | "aws_s3_bucket_objects" => ResourceIcon::Storage,
        "aws_cloudfront_distribution"
        | "aws_cloudfront_function"
        | "aws_cloudfront_origin_access_identity"
        | "aws_cloudfront_origin_access_identities"
        | "aws_cloudfront_origin_access_control"
        | "aws_cloudfront_cache_policy"
        | "aws_cloudfront_origin_request_policy"
        | "aws_cloudfront_response_headers_policy" => ResourceIcon::CloudFront,
        "aws_api_gateway_rest_api"
        | "aws_api_gateway_domain_name"
        | "aws_api_gateway_resource"
        | "aws_api_gateway_vpc_link"
        | "aws_apigatewayv2_api"
        | "aws_apigatewayv2_apis"
        | "aws_apigatewayv2_vpc_link" => ResourceIcon::ApiGateway,
        "aws_iam_role"
        | "aws_iam_roles"
        | "aws_iam_policy"
        | "aws_iam_role_policy"
        | "aws_iam_instance_profile"
        | "aws_iam_instance_profiles"
        | "aws_iam_user"
        | "aws_iam_users"
        | "aws_iam_group"
        | "aws_iam_openid_connect_provider"
        | "aws_iam_saml_provider"
        | "aws_iam_role_policy_attachment" => ResourceIcon::Iam,
        "aws_security_group" | "aws_security_groups" | "aws_network_acl" | "aws_network_acls" => {
            ResourceIcon::Security
        }
        "aws_cloudwatch_log_group" | "aws_cloudwatch_log_groups" | "aws_cloudwatch_metric_alarm" => {
            ResourceIcon::Monitoring
        }
        "aws_autoscaling_group" | "aws_autoscaling_groups" | "aws_launch_configuration" => {
            ResourceIcon::Scaling
        }
        _ => return None,
    })
}
