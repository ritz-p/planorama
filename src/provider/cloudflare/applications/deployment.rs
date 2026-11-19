use super::target::{Kind, Target};
use super::{AttributeReference, Evidence, Failure, Result, collect};
use std::collections::BTreeSet;
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_worker_version", "worker_id") => Some("Worker"),
        ("cloudflare_workers_deployment", "script_name") => Some("Deployed Worker"),
        ("cloudflare_workers_deployment", "versions") => Some("Configured versions"),
        _ => None,
    }
}
pub(super) fn targets(value: Evidence<'_>) -> Result<Vec<Target<'_>>> {
    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = 0.0;
    for (id, percentage) in collect(value.rows()?.into_iter().map(|row| {
        let id = row.text("version_id")?;
        let percentage = row.at("percentage")?;
        percentage.scalar()?;
        Ok((id, percentage.value.as_f64().ok_or(Failure::InvalidValue)?))
    }))? {
        if !(0.0..=100.0).contains(&percentage) || !seen.insert(id) {
            return Err(Failure::IncompleteCollection);
        }
        total += percentage;
        targets.push(Target::new(id, Kind::Version));
    }
    ((total - 100.0).abs() < 0.000001)
        .then_some(targets)
        .ok_or(Failure::IncompleteCollection)
}
pub(super) fn version_owner(
    refs: &[AttributeReference],
    deployment: usize,
    version: usize,
) -> bool {
    let owner = |target, path: &str| {
        refs.iter()
            .find(|r| r.target == target && r.attribute == path && r.service_reference_verified)
            .and_then(|r| {
                if let [source] = r.sources.as_slice() {
                    Some(*source)
                } else {
                    None
                }
            })
    };
    owner(deployment, "script_name")
        .zip(owner(version, "worker_id"))
        .is_some_and(|(a, b)| a == b)
}
pub(super) fn percentages(
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<std::collections::BTreeMap<usize, String>> {
    let values: std::collections::BTreeMap<_, _> = evidence[r.target]
        .at("versions")?
        .rows()?
        .into_iter()
        .map(|row| {
            let id = row.text("version_id")?;
            let percentage = row.at("percentage")?;
            percentage.scalar()?;
            let value = percentage.value.as_f64().ok_or(Failure::InvalidValue)?;
            Ok((
                id,
                if value == 0.0 {
                    "0".into()
                } else {
                    value.to_string()
                },
            ))
        })
        .collect::<Result<_>>()?;
    r.sources
        .iter()
        .map(|&source| {
            let id = evidence[source].text("id")?;
            Ok((
                source,
                values.get(id).cloned().ok_or(Failure::IdentityMismatch)?,
            ))
        })
        .collect()
}
