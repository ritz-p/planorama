pub(crate) mod aws;
pub(crate) mod terraform;

pub(crate) fn has_indexed_service_references(
    provider: &crate::model::ProviderIdentity,
    resource: &str,
) -> bool {
    provider.is_aws() && aws::s3::indexed_resource(resource)
}

pub(crate) fn indexed_service_reference(
    provider: &crate::model::ProviderIdentity,
    resource: &str,
    attribute: &str,
) -> bool {
    provider.is_aws() && aws::s3::indexed_attribute(resource, attribute).is_some()
}

pub(crate) fn reference_label(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
    attribute: &str,
    dependency: bool,
) -> &'static str {
    if dependency {
        "Dependency"
    } else if provider.is_aws() {
        aws::reference_label(resource_type, attribute)
    } else {
        "References"
    }
}

pub(crate) fn placement(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> crate::model::Placement {
    if provider.is_aws() {
        aws::placement(resource_type)
    } else {
        crate::model::Placement::Unresolved
    }
}

pub(crate) fn scope(
    entity: &crate::model::TerraformEntity,
) -> Option<crate::model::architecture::DeploymentScope> {
    entity
        .provider
        .is_aws()
        .then(|| aws::scope(entity))
        .flatten()
}

pub(crate) fn entity_placement(entity: &crate::model::TerraformEntity) -> crate::model::Placement {
    if entity.provider.is_aws() && entity.mode == crate::model::EntityMode::Data {
        aws::data_placement(&entity.resource_type)
    } else {
        placement(&entity.provider, &entity.resource_type)
    }
}

pub(crate) fn classify(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> crate::model::ResourceRole {
    if provider.is_aws() {
        aws::classify(resource_type)
    } else {
        crate::model::ResourceRole::Unknown
    }
}
pub(crate) fn transform(input: crate::semantic::Input) -> crate::model::Graph {
    aws::transform(input)
}
pub(crate) fn diagnostics(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    aws::diagnostics::collect(plan)
}
