use super::ResourceIcon;

impl ResourceIcon {
    pub fn id(self) -> &'static str {
        match self {
            Self::Vpc | Self::Subnet | Self::Gateway | Self::Route | Self::Security => {
                "planorama-icon-aws-vpc"
            }
            Self::Ec2 => "planorama-icon-aws-ec2",
            Self::Endpoint => "planorama-icon-aws-endpoint",
            Self::LoadBalancer => "planorama-icon-aws-elb",
            Self::Lambda => "planorama-icon-aws-lambda",
            Self::Sqs => "planorama-icon-aws-sqs",
            Self::Sns => "planorama-icon-aws-sns",
            Self::EventBridge => "planorama-icon-aws-eventbridge",
            Self::Ecs => "planorama-icon-aws-ecs",
            Self::Eks => "planorama-icon-aws-eks",
            Self::Database => "planorama-icon-aws-rds",
            Self::DynamoDb => "planorama-icon-aws-dynamodb",
            Self::Cache => "planorama-icon-aws-cache",
            Self::Search => "planorama-icon-aws-search",
            Self::Storage => "planorama-icon-aws-s3",
            Self::CloudFront => "planorama-icon-aws-cloudfront",
            Self::Route53 => "planorama-icon-aws-route53",
            Self::Acm => "planorama-icon-aws-acm",
            Self::Kms => "planorama-icon-aws-kms",
            Self::SecretsManager => "planorama-icon-aws-secretsmanager",
            Self::Ssm => "planorama-icon-aws-ssm",
            Self::Efs => "planorama-icon-aws-efs",
            Self::ApiGateway => "planorama-icon-aws-api",
            Self::Iam => "planorama-icon-aws-iam",
            Self::Monitoring => "planorama-icon-aws-monitoring",
            Self::Scaling => "planorama-icon-aws-scaling",
            Self::Terraform => "planorama-icon-terraform",
        }
    }

    pub fn svg(self) -> &'static str {
        match self {
            Self::Vpc | Self::Subnet | Self::Gateway | Self::Route | Self::Security => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/Amazon-Virtual-Private-Cloud.svg"
                ))
            }
            Self::Ec2 => include_str!(concat!(env!("OUT_DIR"), "/Amazon-EC2.svg")),
            Self::Endpoint => include_str!(concat!(env!("OUT_DIR"), "/AWS-PrivateLink.svg")),
            Self::LoadBalancer => {
                include_str!(concat!(env!("OUT_DIR"), "/Elastic-Load-Balancing.svg"))
            }
            Self::Lambda => include_str!(concat!(env!("OUT_DIR"), "/AWS-Lambda.svg")),
            Self::Sqs => include_str!(concat!(env!("OUT_DIR"), "/Amazon-Simple-Queue-Service.svg")),
            Self::EventBridge => include_str!(concat!(env!("OUT_DIR"), "/Amazon-EventBridge.svg")),
            Self::Sns => include_str!(concat!(
                env!("OUT_DIR"),
                "/Amazon-Simple-Notification-Service.svg"
            )),
            Self::Ecs => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/Amazon-Elastic-Container-Service.svg"
                ))
            }
            Self::Eks => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/Amazon-Elastic-Kubernetes-Service.svg"
                ))
            }
            Self::Database => include_str!(concat!(env!("OUT_DIR"), "/Amazon-RDS.svg")),
            Self::DynamoDb => include_str!(concat!(env!("OUT_DIR"), "/Amazon-DynamoDB.svg")),
            Self::Cache => include_str!(concat!(env!("OUT_DIR"), "/Amazon-ElastiCache.svg")),
            Self::Search => {
                include_str!(concat!(env!("OUT_DIR"), "/Amazon-OpenSearch-Service.svg"))
            }
            Self::Storage => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/Amazon-Simple-Storage-Service.svg"
                ))
            }
            Self::CloudFront => include_str!(concat!(env!("OUT_DIR"), "/Amazon-CloudFront.svg")),
            Self::Route53 => include_str!(concat!(env!("OUT_DIR"), "/Amazon-Route-53.svg")),
            Self::Acm => include_str!(concat!(env!("OUT_DIR"), "/AWS-Certificate-Manager.svg")),
            Self::Kms => include_str!(concat!(env!("OUT_DIR"), "/AWS-Key-Management-Service.svg")),
            Self::SecretsManager => {
                include_str!(concat!(env!("OUT_DIR"), "/AWS-Secrets-Manager.svg"))
            }
            Self::Ssm => include_str!(concat!(env!("OUT_DIR"), "/AWS-Systems-Manager.svg")),
            Self::Efs => include_str!(concat!(env!("OUT_DIR"), "/Amazon-EFS.svg")),
            Self::ApiGateway => include_str!(concat!(env!("OUT_DIR"), "/Amazon-API-Gateway.svg")),
            Self::Iam => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/AWS-Identity-and-Access-Management.svg"
                ))
            }
            Self::Monitoring => include_str!(concat!(env!("OUT_DIR"), "/Amazon-CloudWatch.svg")),
            Self::Scaling => include_str!(concat!(env!("OUT_DIR"), "/Amazon-EC2-Auto-Scaling.svg")),
            Self::Terraform => include_str!(concat!(env!("OUT_DIR"), "/terraform-builtin-24.svg")),
        }
    }

    pub fn notice(self) -> &'static str {
        match self {
            Self::Terraform => concat!(
                include_str!("../../assets/icons/terraform/NOTICE.txt"),
                "\n",
                include_str!("../../assets/icons/terraform/LICENSE.txt")
            ),
            _ => include_str!("../../assets/icons/aws/NOTICE.txt"),
        }
    }
}
