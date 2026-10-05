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
        "AWS_VPC",
        "azurerm_virtual_network",
    ] {
        assert_eq!(
            icon_for(Provider::from_resource_type(resource_type), resource_type),
            None
        );
    }
}
