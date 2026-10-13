use crate::model::{ProviderIdentity, architecture::DeploymentScope};

pub(crate) fn label(scope: Option<&DeploymentScope>, provider: &ProviderIdentity) -> String {
    match scope {
        Some(DeploymentScope {
            region: Some(region),
            ..
        }) => format!("Region: {}", region.0),
        Some(DeploymentScope {
            provider,
            region: None,
        }) => format!("{provider} Global"),
        None if provider.is_aws() || matches!(provider.source(), None | Some("" | "unknown")) => {
            "Region: unknown".into()
        }
        None => "Region: not applicable".into(),
    }
}
