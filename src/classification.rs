use crate::model::ResourceRole;

pub(crate) fn classify_entity(entity: &crate::model::TerraformEntity) -> ResourceRole {
    if crate::provider::aws::organizations::unsupported_data(
        &entity.provider,
        &entity.resource_type,
        entity.mode,
    ) {
        ResourceRole::Unknown
    } else if entity.provider.is_aws() && entity.mode == crate::model::EntityMode::Data {
        crate::provider::aws::data_role(&entity.resource_type)
    } else {
        classify(&entity.provider, &entity.resource_type)
    }
}

#[cfg(test)]
#[path = "../tests/unit/classification.rs"]
mod tests;

pub(crate) fn classify(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> ResourceRole {
    crate::provider::classify(provider, resource_type)
}
