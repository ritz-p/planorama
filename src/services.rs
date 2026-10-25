pub(crate) const SHORT_NAMES: &[(&str, &str)] = &[
    ("Amazon EC2", "EC2"),
    ("Amazon S3", "S3"),
    ("AWS Lambda", "Lambda"),
    ("Amazon VPC", "VPC"),
    ("Amazon ECS", "ECS"),
    ("Amazon EKS", "EKS"),
    ("Amazon ECR", "ECR"),
    ("Amazon RDS", "RDS"),
    ("Amazon DynamoDB", "DynamoDB"),
    ("Amazon ElastiCache", "ElastiCache"),
    ("Amazon EFS", "EFS"),
    ("Amazon EBS", "EBS"),
    ("Amazon Simple Queue Service", "SQS"),
    ("Amazon Simple Notification Service", "SNS"),
    ("AWS Identity and Access Management", "IAM"),
    ("AWS Key Management Service", "KMS"),
    ("Elastic Load Balancing", "ELB"),
    ("Amazon CloudWatch", "CloudWatch"),
    ("Amazon Route 53", "Route 53"),
    ("Amazon CloudFront", "CloudFront"),
];

pub(crate) fn short_name(name: &str) -> &str {
    SHORT_NAMES
        .iter()
        .find_map(|(formal, short)| (*formal == name).then_some(*short))
        .unwrap_or(name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VisualRole {
    Execution,
    DataStore,
    Network,
}

pub(crate) fn visual_role(node: &crate::model::Node) -> Option<VisualRole> {
    use crate::model::ResourceRole;
    if !node.provider.is_aws()
        || !matches!(
            node.role,
            ResourceRole::Node | ResourceRole::Connector | ResourceRole::Controller
        )
    {
        return None;
    }
    match node.resource_type.as_str() {
        "aws_instance" | "aws_lambda_function" | "aws_ecs_service" | "aws_eks_cluster" => {
            Some(VisualRole::Execution)
        }
        "aws_s3_bucket"
        | "aws_ebs_volume"
        | "aws_efs_file_system"
        | "aws_db_instance"
        | "aws_rds_cluster"
        | "aws_dynamodb_table"
        | "aws_elasticache_cluster"
        | "aws_elasticache_serverless_cache"
        | "aws_elasticache_replication_group"
        | "aws_redshift_cluster"
        | "aws_opensearch_domain" => Some(VisualRole::DataStore),
        "aws_nat_gateway"
        | "aws_internet_gateway"
        | "aws_vpc_endpoint"
        | "aws_lb"
        | "aws_cloudfront_distribution"
        | "aws_apigatewayv2_api"
        | "aws_api_gateway_rest_api" => Some(VisualRole::Network),
        _ => None,
    }
}
