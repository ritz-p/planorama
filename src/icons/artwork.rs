use super::ResourceIcon;

impl ResourceIcon {
    pub fn id(self) -> &'static str {
        match self {
            Self::Vpc => "planorama-icon-aws-vpc",
            Self::Subnet => "planorama-icon-aws-subnet",
            Self::Ec2 => "planorama-icon-aws-ec2",
            Self::Gateway => "planorama-icon-aws-gateway",
            Self::Route => "planorama-icon-aws-route",
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
            Self::Security => "planorama-icon-aws-security",
            Self::Monitoring => "planorama-icon-aws-monitoring",
            Self::Scaling => "planorama-icon-aws-scaling",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Self::Vpc
            | Self::Subnet
            | Self::Gateway
            | Self::Route
            | Self::Endpoint
            | Self::LoadBalancer
            | Self::CloudFront
            | Self::ApiGateway => "#6346a6",
            Self::Ec2 | Self::Lambda | Self::Ecs | Self::Eks | Self::Scaling => "#a54e08",
            Self::Database | Self::DynamoDb | Self::Cache | Self::Search => "#285bb5",
            Self::Storage => "#287446",
            Self::Iam | Self::Security => "#a53655",
            Self::Monitoring => "#9a3f86",
        }
    }

    pub fn shapes(self) -> &'static str {
        match self {
            Self::Vpc => {
                r#"<rect x="5" y="6" width="14" height="12" rx="3"/><circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/><path d="M10.5 12h3M12 3v3m0 12v3"/>"#
            }
            Self::Subnet => {
                r#"<rect x="5" y="5" width="14" height="14" rx="2"/><path d="M5 12h14M12 5v14"/>"#
            }
            Self::Ec2 => {
                r#"<rect x="7" y="7" width="10" height="10" rx="1"/><path d="M9 3v4m6-4v4M9 17v4m6-4v4M3 9h4m-4 6h4m10-6h4m-4 6h4"/>"#
            }
            Self::Gateway => r#"<path d="M8 5h8v14H8zM3 9h8m-3-3 3 3-3 3m13 3h-8m3-3-3 3 3 3"/>"#,
            Self::Route => {
                r#"<path d="M5 18h5V6h8m-3-3 3 3-3 3M10 13h8m-3-3 3 3-3 3"/><circle cx="5" cy="18" r="2"/>"#
            }
            Self::Endpoint => {
                r#"<path d="M4 12h6m4 0h6M7 9l3 3-3 3m10-6-3 3 3 3"/><rect x="10" y="5" width="4" height="14" rx="1"/>"#
            }
            Self::LoadBalancer => {
                r#"<path d="M4 12h7m0 0 6-6m-6 6 6 6"/><rect x="17" y="4" width="4" height="4" rx="1"/><rect x="17" y="16" width="4" height="4" rx="1"/><circle cx="5" cy="12" r="2"/>"#
            }
            Self::Lambda => r#"<path d="M6 5h5l7 14h3M11 8 5 19m9-7-3 7"/>"#,
            Self::Ecs => r#"<path d="m12 4 8 4v8l-8 4-8-4V8zm-8 4 8 4 8-4m-8 4v8"/>"#,
            Self::Eks => r#"<path d="m12 3 8 5v8l-8 5-8-5V8zM9 8v8m6-8-6 4 6 4"/>"#,
            Self::Database => {
                r#"<ellipse cx="12" cy="6" rx="7" ry="3"/><path d="M5 6v12c0 4 14 4 14 0V6M5 12c0 4 14 4 14 0"/>"#
            }
            Self::DynamoDb => {
                r#"<rect x="4" y="5" width="16" height="14" rx="2"/><path d="M4 10h16M9 5v14m6-14v14"/>"#
            }
            Self::Cache => r#"<path d="M5 6h14M5 18h14M13 3l-6 10h5l-1 8 7-11h-5z"/>"#,
            Self::Search => {
                r#"<circle cx="10" cy="10" r="6"/><path d="m15 15 6 6M7 10h6m-3-3v6"/>"#
            }
            Self::Storage => {
                r#"<ellipse cx="12" cy="6" rx="8" ry="3"/><path d="m4 6 2 12c0 4 12 4 12 0l2-12M9 13h6"/>"#
            }
            Self::CloudFront => {
                r#"<circle cx="12" cy="12" r="8"/><ellipse cx="12" cy="12" rx="3" ry="8"/><path d="M4 12h16M6 7h12M6 17h12"/>"#
            }
            Self::ApiGateway => r#"<path d="m8 6-5 6 5 6m8-12 5 6-5 6M14 4l-4 16"/>"#,
            Self::Iam => {
                r#"<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>"#
            }
            Self::Security => r#"<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6zM8 12l3 3 5-6"/>"#,
            Self::Monitoring => {
                r#"<rect x="3" y="4" width="18" height="14" rx="2"/><path d="m5 13 4-5 4 6 5-6M9 21h6m-3-3v3"/>"#
            }
            Self::Scaling => {
                r#"<rect x="8" y="8" width="8" height="8" rx="1"/><path d="M3 8V3h5M3 3l5 5m13 8v5h-5m5 0-5-5"/>"#
            }
        }
    }
}
