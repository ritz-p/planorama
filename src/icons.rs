mod artwork;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconMode {
    #[default]
    Icon,
    ServiceName,
}

pub(crate) const SIZE: usize = 24;
pub(crate) const GAP: usize = 8;

#[cfg(test)]
#[path = "../tests/unit/icons.rs"]
mod tests;

#[derive(Clone, Copy)]
pub enum Provider {
    Aws,
    Terraform,
    Other,
}

impl Provider {
    pub fn from_identity(provider: &crate::model::ProviderIdentity) -> Self {
        if provider.is_aws() {
            Self::Aws
        } else if provider.is_terraform() {
            Self::Terraform
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceIcon {
    Vpc,
    Subnet,
    Ec2,
    Gateway,
    Route,
    Endpoint,
    LoadBalancer,
    Lambda,
    Sqs,
    Sns,
    EventBridge,
    Ecs,
    Eks,
    Database,
    DynamoDb,
    Cache,
    Search,
    Storage,
    CloudFront,
    Route53,
    Acm,
    Kms,
    SecretsManager,
    Ssm,
    Efs,
    Ebs,
    ApiGateway,
    Iam,
    Security,
    Monitoring,
    Scaling,
    Terraform,
}

impl ResourceIcon {
    pub fn service_name(self) -> &'static str {
        match self {
            Self::Vpc | Self::Subnet | Self::Gateway | Self::Route | Self::Security => "Amazon VPC",
            Self::Ec2 => "Amazon EC2",
            Self::Endpoint => "Amazon VPC",
            Self::LoadBalancer => "Elastic Load Balancing",
            Self::Lambda => "AWS Lambda",
            Self::Sqs => "Amazon Simple Queue Service",
            Self::Sns => "Amazon Simple Notification Service",
            Self::EventBridge => "Amazon EventBridge",
            Self::Ecs => "Amazon ECS",
            Self::Eks => "Amazon EKS",
            Self::Database => "Amazon RDS",
            Self::DynamoDb => "Amazon DynamoDB",
            Self::Cache => "Amazon ElastiCache",
            Self::Search => "Amazon OpenSearch Service",
            Self::Storage => "Amazon S3",
            Self::CloudFront => "Amazon CloudFront",
            Self::Route53 => "Amazon Route 53",
            Self::Acm => "AWS Certificate Manager",
            Self::Kms => "AWS Key Management Service",
            Self::SecretsManager => "AWS Secrets Manager",
            Self::Ssm => "AWS Systems Manager",
            Self::Efs => "Amazon EFS",
            Self::Ebs => "Amazon EBS",
            Self::ApiGateway => "Amazon API Gateway",
            Self::Iam => "AWS Identity and Access Management",
            Self::Monitoring => "Amazon CloudWatch",
            Self::Scaling => "Amazon EC2 Auto Scaling",
            Self::Terraform => "Terraform built-in",
        }
    }
}

pub fn icon_for(provider: Provider, resource_type: &str) -> Option<ResourceIcon> {
    match provider {
        Provider::Aws => crate::provider::aws::icon_for(resource_type),
        Provider::Terraform => crate::provider::terraform::icon_for(resource_type),
        Provider::Other => None,
    }
}

pub(crate) fn for_node(node: &crate::model::Node) -> Option<ResourceIcon> {
    icon_for(Provider::from_identity(&node.provider), &node.resource_type)
}
