use crate::model::ResourceRole;

#[cfg(test)]
#[path = "../../tests/unit/classification/aws.rs"]
mod tests;

pub(super) fn classify(resource_type: &str) -> ResourceRole {
    match resource_type {
        "aws_vpc" | "aws_subnet" => ResourceRole::Container,
        "aws_instance"
        | "aws_ami"
        | "aws_eip"
        | "aws_db_instance"
        | "aws_db_subnet_group"
        | "aws_lambda_function"
        | "aws_rds_cluster"
        | "aws_dynamodb_table"
        | "aws_elasticache_cluster"
        | "aws_opensearch_domain"
        | "aws_s3_bucket"
        | "aws_cloudwatch_log_group" => ResourceRole::Node,
        "aws_vpc_peering_connection"
        | "aws_internet_gateway"
        | "aws_nat_gateway"
        | "aws_route_table"
        | "aws_route"
        | "aws_vpc_endpoint"
        | "aws_ec2_transit_gateway"
        | "aws_ec2_transit_gateway_route_table"
        | "aws_ec2_transit_gateway_route"
        | "aws_lb"
        | "aws_lb_listener"
        | "aws_cloudfront_distribution"
        | "aws_api_gateway_rest_api"
        | "aws_apigatewayv2_api" => ResourceRole::Connector,
        "aws_route_table_association"
        | "aws_lb_target_group_attachment"
        | "aws_ec2_transit_gateway_vpc_attachment"
        | "aws_ec2_transit_gateway_peering_attachment"
        | "aws_ec2_transit_gateway_route_table_association"
        | "aws_ec2_transit_gateway_route_table_propagation"
        | "aws_vpc_endpoint_route_table_association"
        | "aws_vpc_endpoint_subnet_association"
        | "aws_iam_role_policy_attachment" => ResourceRole::Association,
        "aws_security_group"
        | "aws_network_acl"
        | "aws_iam_role"
        | "aws_iam_policy"
        | "aws_iam_role_policy"
        | "aws_iam_instance_profile"
        | "aws_lb_listener_rule" => ResourceRole::Policy,
        "aws_autoscaling_group"
        | "aws_ecs_service"
        | "aws_ecs_cluster"
        | "aws_ecs_task_definition"
        | "aws_eks_cluster"
        | "aws_eks_node_group"
        | "aws_lb_target_group"
        | "aws_cloudwatch_metric_alarm" => ResourceRole::Controller,
        _ => ResourceRole::Unknown,
    }
}
