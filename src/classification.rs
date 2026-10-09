use crate::model::ResourceRole;

#[cfg(test)]
#[path = "../tests/unit/classification.rs"]
mod tests;

pub(crate) fn classify(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> ResourceRole {
    crate::provider::classify(provider, resource_type)
}
