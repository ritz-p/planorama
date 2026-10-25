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

pub(crate) fn display_name(node: &crate::model::Node) -> Option<&str> {
    let service = node.service_name?;
    if node.provider.is_aws() {
        let name = match node.resource_type.as_str() {
            "aws_ami" => "AMI",
            "aws_launch_template" => "Launch Template",
            "aws_launch_configuration" => "Launch Configuration",
            "aws_autoscaling_group" => "Auto Scaling Group",
            "aws_ebs_snapshot" => "EBS Snapshot",
            "aws_efs_access_point" => "EFS Access Point",
            "aws_efs_mount_target" => "EFS Mount Target",
            "aws_kms_alias" => "KMS Alias",
            "aws_secretsmanager_secret_version" => "Secret Version",
            "aws_cloudwatch_log_group" => "Log Group",
            "aws_cloudwatch_log_stream" => "Log Stream",
            "aws_sns_topic_subscription" => "SNS Subscription",
            "aws_vpc" => "VPC",
            "aws_subnet" => "Subnet",
            "aws_route_table" | "aws_default_route_table" => "Route Table",
            "aws_route" => "Route",
            "aws_route_table_association" => "Route Table Association",
            "aws_nat_gateway" => "NAT Gateway",
            "aws_internet_gateway" => "Internet Gateway",
            "aws_egress_only_internet_gateway" => "Egress-only Gateway",
            "aws_vpc_endpoint" => "VPC Endpoint",
            "aws_vpc_endpoint_service" => "VPC Endpoint Service",
            "aws_vpc_endpoint_route_table_association" => "Endpoint Route Table Association",
            "aws_vpc_endpoint_subnet_association" => "Endpoint Subnet Association",
            "aws_vpc_peering_connection" => "VPC Peering",
            "aws_security_group" | "aws_default_security_group" => "Security Group",
            "aws_security_group_rule"
            | "aws_vpc_security_group_ingress_rule"
            | "aws_vpc_security_group_egress_rule" => "Security Group Rule",
            "aws_network_acl" | "aws_default_network_acl" => "Network ACL",
            "aws_network_acl_rule" => "Network ACL Rule",
            "aws_network_interface" => "Network Interface",
            "aws_eip" => "Elastic IP",
            "aws_lb" | "aws_alb" | "aws_elb" => "Load Balancer",
            "aws_lb_listener" | "aws_alb_listener" => "Load Balancer Listener",
            "aws_lb_listener_rule" | "aws_alb_listener_rule" => "Listener Rule",
            "aws_lb_target_group" | "aws_alb_target_group" => "Target Group",
            "aws_lb_target_group_attachment" | "aws_alb_target_group_attachment" => {
                "Target Attachment"
            }
            "aws_ecs_cluster" => "ECS Cluster",
            "aws_ecs_service" => "ECS Service",
            "aws_ecs_task_definition" => "ECS Task Definition",
            "aws_eks_cluster" => "EKS Cluster",
            "aws_eks_node_group" => "EKS Node Group",
            "aws_db_instance" => "RDS Instance",
            "aws_rds_cluster" => "RDS Cluster",
            "aws_rds_cluster_instance" => "RDS Cluster Instance",
            "aws_db_subnet_group" => "DB Subnet Group",
            "aws_db_proxy" => "RDS Proxy",
            "aws_s3_bucket_policy" => "S3 Bucket Policy",
            "aws_s3_bucket_notification" => "S3 Notification",
            "aws_s3_bucket_replication_configuration" => "S3 Replication",
            "aws_iam_user" => "IAM User",
            "aws_iam_role" => "IAM Role",
            "aws_iam_group" => "IAM Group",
            "aws_iam_policy" => "IAM Policy",
            "aws_iam_user_policy" => "IAM User Policy",
            "aws_iam_role_policy" => "IAM Role Policy",
            "aws_iam_group_policy" => "IAM Group Policy",
            "aws_iam_user_policy_attachment" => "IAM User Policy Attachment",
            "aws_iam_role_policy_attachment" => "IAM Role Policy Attachment",
            "aws_iam_group_policy_attachment" => "IAM Group Policy Attachment",
            "aws_iam_policy_attachment" => "IAM Policy Attachment",
            "aws_iam_instance_profile" => "IAM Instance Profile",
            "aws_iam_access_key" => "IAM Access Key",
            "aws_iam_openid_connect_provider" => "IAM OIDC Provider",
            "aws_iam_saml_provider" => "IAM SAML Provider",
            _ => return Some(short_name(service)),
        };
        return Some(name);
    }
    Some(short_name(service))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Compute,
    Storage,
    Network,
    Iam,
    Neutral,
}

impl Category {
    pub(crate) fn palette(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Compute => ("compute", "#ffedd5", "#7c2d12"),
            Self::Storage => ("storage", "#dcfce7", "#14532d"),
            Self::Network => ("network", "#dbeafe", "#1e3a8a"),
            Self::Iam => ("iam", "#fee2e2", "#7f1d1d"),
            Self::Neutral => ("neutral", "#e2e8f0", "#0f172a"),
        }
    }
}

pub(crate) fn category(node: &crate::model::Node) -> Category {
    if !node.provider.is_aws() {
        return Category::Neutral;
    }
    match node.service_name {
        Some("AWS Identity and Access Management") => Category::Iam,
        Some(
            "Amazon EC2" | "AWS Lambda" | "Amazon ECS" | "Amazon EKS" | "Amazon EC2 Auto Scaling",
        ) => Category::Compute,
        Some("Amazon S3" | "Amazon EBS" | "Amazon EFS") => Category::Storage,
        Some(
            "Amazon VPC"
            | "Elastic Load Balancing"
            | "Amazon Route 53"
            | "Amazon CloudFront"
            | "Amazon API Gateway",
        ) => Category::Network,
        _ => Category::Neutral,
    }
}
