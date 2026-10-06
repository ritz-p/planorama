mod aws;

use crate::model::ResourceRole;

#[cfg(test)]
#[path = "../tests/unit/classification.rs"]
mod tests;

pub(crate) fn classify(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> ResourceRole {
    match provider.is_aws() {
        true => aws::classify(resource_type),
        false => ResourceRole::Unknown,
    }
}
