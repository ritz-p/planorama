pub(crate) mod aws;

pub(crate) fn scope(
    entity: &crate::model::TerraformEntity,
) -> Option<crate::model::architecture::DeploymentScope> {
    entity
        .provider
        .is_aws()
        .then(|| aws::scope(entity))
        .flatten()
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
