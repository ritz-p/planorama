mod artwork;

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
    Terraform,
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
