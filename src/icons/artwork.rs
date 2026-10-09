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
            Self::Ecs => "planorama-icon-aws-ecs",
            Self::Eks => "planorama-icon-aws-eks",
            Self::Database => "planorama-icon-aws-rds",
            Self::DynamoDb => "planorama-icon-aws-dynamodb",
            Self::Cache => "planorama-icon-aws-cache",
            Self::Search => "planorama-icon-aws-search",
            Self::Storage => "planorama-icon-aws-s3",
            Self::CloudFront => "planorama-icon-aws-cloudfront",
            Self::ApiGateway => "planorama-icon-aws-api",
            Self::Iam => "planorama-icon-aws-iam",
            Self::Monitoring => "planorama-icon-aws-monitoring",
            Self::Scaling => "planorama-icon-aws-scaling",
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
            Self::ApiGateway => include_str!(concat!(env!("OUT_DIR"), "/Amazon-API-Gateway.svg")),
            Self::Iam => {
                include_str!(concat!(
                    env!("OUT_DIR"),
                    "/AWS-Identity-and-Access-Management.svg"
                ))
            }
            Self::Monitoring => include_str!(concat!(env!("OUT_DIR"), "/Amazon-CloudWatch.svg")),
            Self::Scaling => include_str!(concat!(env!("OUT_DIR"), "/Amazon-EC2-Auto-Scaling.svg")),
        }
    }
}
