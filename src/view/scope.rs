use crate::model::{ProviderIdentity, architecture::DeploymentScope};

pub(crate) fn component<'a>(
    members: impl Iterator<Item = Option<&'a crate::model::Node>>,
) -> String {
    let mut labels = std::collections::BTreeSet::new();
    let mut regions = std::collections::BTreeSet::new();
    let mut accounts = std::collections::BTreeSet::new();
    for member in members {
        let Some(node) = member else {
            labels.insert("Member unavailable in this view".to_owned());
            continue;
        };
        labels.insert(label(node.entity.scope.as_ref(), &node.provider));
        if let Some(scope) = &node.entity.scope {
            if let Some(region) = &scope.region {
                regions.insert(&region.0);
            }
            accounts.insert(scope.account.as_ref());
        } else {
            accounts.insert(None);
        }
    }
    let mut qualifiers = Vec::new();
    if regions.len() > 1 {
        qualifiers.push("Multi-region");
    }
    if accounts.iter().flatten().count() > 1 {
        qualifiers.push("Multi-account");
    }
    if accounts.contains(&None) && accounts.len() > 1 {
        qualifiers.push("Account partially unknown");
    }
    if labels.len() > 1 {
        qualifiers.push("Mixed member scope");
    }
    if labels.is_empty() {
        return "Member scope: unavailable".into();
    }
    let prefix = if qualifiers.is_empty() {
        "Member scope".to_owned()
    } else {
        qualifiers.join(" · ")
    };
    format!(
        "{prefix}: {}",
        labels.into_iter().collect::<Vec<_>>().join("; ")
    )
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
