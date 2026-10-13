use crate::model::{ProviderIdentity, architecture::DeploymentScope};

pub(crate) fn panel_label(scope: &DeploymentScope) -> String {
    let region = scope.region.as_ref().map_or(
        if scope.global {
            "Global"
        } else {
            "Region: unknown"
        },
        |r| r.0.as_str(),
    );
    let label = format!("{} {region}", scope.provider);
    match &scope.account {
        Some(account) => format!("{label} / Account: {}", account.as_str()),
        None => label,
    }
}

pub(crate) fn label(scope: Option<&DeploymentScope>, provider: &ProviderIdentity) -> String {
    let region_label = match scope {
        Some(DeploymentScope {
            region: Some(region),
            ..
        }) => format!("Region: {}", region.0),
        Some(DeploymentScope {
            provider,
            region: None,
            global: true,
            ..
        }) => format!("{provider} Global"),
        Some(_) => "Region: unknown".into(),
        None if provider.is_aws() || matches!(provider.source(), None | Some("" | "unknown")) => {
            "Region: unknown".into()
        }
        None => "Region: not applicable".into(),
    };
    match scope.and_then(|s| s.account.as_ref()) {
        Some(account) => format!("AWS Account: {} · {region_label}", account.as_str()),
        None => region_label,
    }
}
