mod artwork;

macro_rules! service_icons {
    ($($variant:ident => ($file:literal, $id:literal, $name:literal),)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum ServiceIcon { $($variant,)* }
        impl ServiceIcon {
            pub(super) fn id(self) -> &'static str {
                match self { $(Self::$variant => $id,)* }
            }
            pub(super) fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)* }
            }
        }
    };
}

mod services;
pub use services::ServiceIcon;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconMode {
    Icon,
    #[default]
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
    Service(ServiceIcon),
    Cognito,
    StepFunctions,
    Redshift,
    CloudTrail,
    Backup,
    Organizations,
    Vpc,
    Subnet,
    Ec2,
    Gateway,
    Route,
    Endpoint,
    LoadBalancer,
    Lambda,
    Sqs,
    Kinesis,
    Firehose,
    Glue,
    Athena,
    Sns,
    EventBridge,
    Ecs,
    Ecr,
    Eks,
    Database,
    DynamoDb,
    Cache,
    Search,
    Storage,
    CloudFront,
    Waf,
    Route53,
    Acm,
    PrivateCa,
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
            Self::Service(icon) => icon.name(),
            Self::Cognito => "Amazon Cognito",
            Self::StepFunctions => "AWS Step Functions",
            Self::Redshift => "Amazon Redshift",
            Self::CloudTrail => "AWS CloudTrail",
            Self::Backup => "AWS Backup",
            Self::Organizations => "AWS Organizations",
            Self::Vpc | Self::Subnet | Self::Gateway | Self::Route | Self::Security => "Amazon VPC",
            Self::Ec2 => "Amazon EC2",
            Self::Endpoint => "Amazon VPC",
            Self::LoadBalancer => "Elastic Load Balancing",
            Self::Lambda => "AWS Lambda",
            Self::Sqs => "Amazon Simple Queue Service",
            Self::Kinesis => "Amazon Kinesis Data Streams",
            Self::Firehose => "Amazon Data Firehose",
            Self::Glue => "AWS Glue",
            Self::Athena => "Amazon Athena",
            Self::Sns => "Amazon Simple Notification Service",
            Self::EventBridge => "Amazon EventBridge",
            Self::Ecs => "Amazon ECS",
            Self::Ecr => "Amazon ECR",
            Self::Eks => "Amazon EKS",
            Self::Database => "Amazon RDS",
            Self::DynamoDb => "Amazon DynamoDB",
            Self::Cache => "Amazon ElastiCache",
            Self::Search => "Amazon OpenSearch Service",
            Self::Storage => "Amazon S3",
            Self::CloudFront => "Amazon CloudFront",
            Self::Waf => "AWS WAF",
            Self::Route53 => "Amazon Route 53",
            Self::Acm => "AWS Certificate Manager",
            Self::PrivateCa => "AWS Private CA",
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
    if node.provider.is_aws() && node.mode == crate::model::EntityMode::Data {
        return crate::provider::aws::data_icon(&node.resource_type);
    }
    icon_for(Provider::from_identity(&node.provider), &node.resource_type)
}
