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
