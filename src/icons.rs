mod artwork;
mod aws;

#[cfg(test)]
#[path = "../tests/unit/icons.rs"]
mod tests;

#[derive(Clone, Copy)]
pub enum Provider {
    Aws,
    Other,
}

impl Provider {
    pub fn from_resource_type(resource_type: &str) -> Self {
        match resource_type.strip_prefix("aws_") {
            Some(_) => Self::Aws,
            None => Self::Other,
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
    Ecs,
    Eks,
    Database,
    DynamoDb,
    Cache,
    Search,
    Storage,
    CloudFront,
    ApiGateway,
    Iam,
    Security,
    Monitoring,
    Scaling,
}

pub fn icon_for(provider: Provider, resource_type: &str) -> Option<ResourceIcon> {
    match provider {
        Provider::Aws => aws::icon_for(resource_type),
        Provider::Other => None,
    }
}
