use super::datazone::{self, DOMAIN, PROJECT, TERM, current, same_domain, string};
use crate::model::{AttributeReference, AwsAccountScope, Graph, RegionScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/datazone_environments.rs"]
mod tests;
pub(super) const ENVIRONMENT: &str = "aws_datazone_environment";
pub(super) const PROFILE: &str = "aws_datazone_environment_profile";
pub(super) const CONFIGURATION: &str = "aws_datazone_environment_blueprint_configuration";
pub(super) fn resource(k: &str) -> bool {
    matches!(k, ENVIRONMENT | PROFILE | CONFIGURATION)
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (ENVIRONMENT | PROFILE, "domain_identifier") | (CONFIGURATION, "domain_id") => {
            Some("DataZone domain")
        }
        (ENVIRONMENT | PROFILE, "project_identifier") => Some("DataZone project"),
        (ENVIRONMENT, "profile_identifier") => Some("DataZone environment profile"),
        (ENVIRONMENT, "glossary_terms") => Some("DataZone environment glossary terms"),
        (ENVIRONMENT, "blueprint_identifier")
        | (PROFILE, "environment_blueprint_identifier")
        | (CONFIGURATION, "environment_blueprint_id") => Some("DataZone blueprint lookup"),
        (CONFIGURATION, "manage_access_role_arn") => Some("Blueprint access role"),
        (CONFIGURATION, "provisioning_role_arn") => Some("Blueprint provisioning role"),
        _ => None,
    }
}
pub(super) fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (ENVIRONMENT | PROFILE, "domain_identifier", DOMAIN)
        | (CONFIGURATION, "domain_id", DOMAIN)
        | (ENVIRONMENT | PROFILE, "project_identifier", PROJECT)
        | (ENVIRONMENT, "profile_identifier", PROFILE)
        | (ENVIRONMENT, "glossary_terms", TERM) => &["id"],
        (CONFIGURATION, "manage_access_role_arn" | "provisioning_role_arn", "aws_iam_role") => {
            &["arn"]
        }
        _ => &[],
    }
}
fn target_paths(k: &str) -> Option<(&'static str, &'static str)> {
    match k {
        ENVIRONMENT => Some(("account_identifier", "account_region")),
        PROFILE => Some(("aws_account_id", "aws_account_region")),
        _ => None,
    }
}
fn target_scope(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<(AwsAccountScope, RegionScope)> {
    if !current(&nodes[t]) {
        return None;
    }
    let (account, region) = target_paths(&nodes[t].resource_type)?;
    super::lambda_core::unique(refs, t, account)?;
    super::lambda_core::unique(refs, t, region)?;
    let account = AwsAccountScope::new(string(e, t, account)?)?;
    let region = string(e, t, region)?;
    let parts: Vec<_> = region.split('-').collect();
    if parts.len() != 3
        || parts[0].len() != 2
        || !(4..=10).contains(&parts[1].len())
        || !parts[..2]
            .iter()
            .all(|p| p.bytes().all(|b| b.is_ascii_lowercase()))
        || parts[2].len() != 1
        || !parts[2].bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((account, RegionScope(region.into())))
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            label(k, &r.attribute)?;
            let p = r.attribute.as_str();
            let shape = if p == "glossary_terms" {
                e[r.target].0[p].as_array().is_some_and(|a| {
                    !a.is_empty() && a.len() <= 20 && a.iter().all(Value::is_string)
                })
            } else {
                e[r.target].0[p].is_string()
            };
            Some(
                shape
                    && super::nested_references::verify(nodes, refs, e, r, fields, |s, _| {
                        matches!(p, "domain_identifier" | "domain_id")
                            || k == CONFIGURATION
                            || same_domain(nodes, refs, e, r.target, s)
                    })
                    .is_some(),
            )
        })
        .collect();
    let targets: Vec<_> = (0..nodes.len())
        .map(|t| target_scope(nodes, refs, e, t))
        .collect();
    for (r, ok) in refs.iter_mut().zip(results) {
        if let Some(ok) = ok {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
        }
        if target_paths(&nodes[r.target].resource_type).is_some_and(|(a, _)| a == r.attribute) {
            if let Some((account, region)) = &targets[r.target] {
                r.service_reference_owner_account = Some(account.clone());
                r.service_reference_region = Some(region.clone());
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .all(|&s| datazone::compatible(raw, r.target, s))
        && (matches!(r.attribute.as_str(), "domain_identifier" | "domain_id")
            || (datazone::domain_bound(raw, r.target)
                && (k == CONFIGURATION
                    || r.sources.iter().all(|&s| datazone::domain_bound(raw, s)))))
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for r in &raw.attributes {
        let node = &raw.nodes[r.target];
        if !node.provider.is_aws()
            || node.mode != crate::model::EntityMode::Managed
            || node.action == crate::model::Action::Delete
            || node.deposed_key.is_some()
            || target_paths(&node.resource_type).is_none_or(|(a, _)| a != r.attribute)
        {
            continue;
        }
        if let Some((account, region)) = r
            .service_reference_owner_account
            .as_ref()
            .zip(r.service_reference_region.as_ref())
        {
            graph.nodes[r.target].entity.deployment_target =
                Some(crate::model::architecture::DeploymentScope {
                    provider: "AWS".into(),
                    account: Some(account.clone()),
                    region: Some(region.clone()),
                    global: false,
                });
        }
    }
}
