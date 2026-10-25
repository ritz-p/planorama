use super::ResourceIcon;

impl ResourceIcon {
    pub fn id(self) -> &'static str {
        match self {
            Self::Service(icon) => icon.id(),
            Self::Cognito => "planorama-icon-aws-cognito",
            Self::StepFunctions => "planorama-icon-aws-step-functions",
            Self::Redshift => "planorama-icon-aws-redshift",
            Self::CloudTrail => "planorama-icon-aws-cloudtrail",
            Self::Backup => "planorama-icon-aws-backup",
            Self::Organizations => "planorama-icon-aws-organizations",
            Self::Vpc | Self::Subnet | Self::Gateway | Self::Route | Self::Security => {
                "planorama-icon-aws-vpc"
            }
            Self::Ec2 => "planorama-icon-aws-ec2",
            Self::Endpoint => "planorama-icon-aws-endpoint",
            Self::LoadBalancer => "planorama-icon-aws-elb",
            Self::Lambda => "planorama-icon-aws-lambda",
            Self::Sqs => "planorama-icon-aws-sqs",
            Self::Kinesis => "planorama-icon-aws-kinesis",
            Self::Firehose => "planorama-icon-aws-firehose",
            Self::Glue => "planorama-icon-aws-glue",
            Self::Athena => "planorama-icon-aws-athena",
            Self::Sns => "planorama-icon-aws-sns",
            Self::EventBridge => "planorama-icon-aws-eventbridge",
            Self::Ecs => "planorama-icon-aws-ecs",
            Self::Ecr => "planorama-icon-aws-ecr",
            Self::Eks => "planorama-icon-aws-eks",
            Self::Database => "planorama-icon-aws-rds",
            Self::DynamoDb => "planorama-icon-aws-dynamodb",
            Self::Cache => "planorama-icon-aws-cache",
            Self::Search => "planorama-icon-aws-search",
            Self::Storage => "planorama-icon-aws-s3",
            Self::CloudFront => "planorama-icon-aws-cloudfront",
            Self::Waf => "planorama-icon-aws-waf",
            Self::Route53 => "planorama-icon-aws-route53",
            Self::Acm => "planorama-icon-aws-acm",
            Self::PrivateCa => "planorama-icon-aws-private-ca",
            Self::Kms => "planorama-icon-aws-kms",
            Self::SecretsManager => "planorama-icon-aws-secretsmanager",
            Self::Ssm => "planorama-icon-aws-ssm",
            Self::Efs => "planorama-icon-aws-efs",
            Self::Ebs => "planorama-icon-aws-ebs",
            Self::ApiGateway => "planorama-icon-aws-api",
            Self::Iam => "planorama-icon-aws-iam",
            Self::Monitoring => "planorama-icon-aws-monitoring",
            Self::Scaling => "planorama-icon-aws-scaling",
            Self::Terraform => "planorama-icon-terraform",
        }
    }

    pub fn svg(self) -> &'static str {
        ""
    }

    pub fn notice(self) -> &'static str {
        ""
    }
}
