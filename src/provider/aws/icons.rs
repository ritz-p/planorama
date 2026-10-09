use crate::icons::ResourceIcon;

pub(super) fn icon_for(resource_type: &str) -> Option<ResourceIcon> {
    Some(match resource_type {
        "aws_vpc" | "aws_vpc_peering_connection" => ResourceIcon::Vpc,
        "aws_subnet" => ResourceIcon::Subnet,
        "aws_instance" | "aws_ami" | "aws_eip" => ResourceIcon::Ec2,
        "aws_internet_gateway"
        | "aws_nat_gateway"
        | "aws_ec2_transit_gateway"
        | "aws_ec2_transit_gateway_vpc_attachment"
        | "aws_ec2_transit_gateway_peering_attachment" => ResourceIcon::Gateway,
        "aws_route_table"
        | "aws_route"
        | "aws_route_table_association"
        | "aws_ec2_transit_gateway_route_table"
        | "aws_ec2_transit_gateway_route"
        | "aws_ec2_transit_gateway_route_table_association"
        | "aws_ec2_transit_gateway_route_table_propagation" => ResourceIcon::Route,
        "aws_vpc_endpoint"
        | "aws_vpc_endpoint_route_table_association"
        | "aws_vpc_endpoint_subnet_association" => ResourceIcon::Endpoint,
        "aws_lb"
        | "aws_lb_listener"
        | "aws_lb_listener_rule"
        | "aws_lb_target_group"
        | "aws_lb_target_group_attachment" => ResourceIcon::LoadBalancer,
        "aws_lambda_function" => ResourceIcon::Lambda,
        "aws_ecs_cluster" | "aws_ecs_service" | "aws_ecs_task_definition" => ResourceIcon::Ecs,
        "aws_eks_cluster" | "aws_eks_node_group" => ResourceIcon::Eks,
        "aws_db_instance" | "aws_db_subnet_group" | "aws_rds_cluster" => ResourceIcon::Database,
        "aws_dynamodb_table" => ResourceIcon::DynamoDb,
        "aws_elasticache_cluster" => ResourceIcon::Cache,
        "aws_opensearch_domain" => ResourceIcon::Search,
        "aws_s3_bucket" => ResourceIcon::Storage,
        "aws_cloudfront_distribution" => ResourceIcon::CloudFront,
        "aws_api_gateway_rest_api" | "aws_apigatewayv2_api" => ResourceIcon::ApiGateway,
        "aws_iam_role"
        | "aws_iam_policy"
        | "aws_iam_role_policy"
        | "aws_iam_instance_profile"
        | "aws_iam_role_policy_attachment" => ResourceIcon::Iam,
        "aws_security_group" | "aws_network_acl" => ResourceIcon::Security,
        "aws_cloudwatch_log_group" | "aws_cloudwatch_metric_alarm" => ResourceIcon::Monitoring,
        "aws_autoscaling_group" => ResourceIcon::Scaling,
        _ => return None,
    })
}
