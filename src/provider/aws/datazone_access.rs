use super::datazone::{self, ASSET, DOMAIN, PROJECT, same_domain, string, value};
use super::datazone_environments::{CONFIGURATION, PROFILE};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/datazone_access.rs"]
mod tests;
pub(super) const USER: &str = "aws_datazone_user_profile";
pub(super) const GRANT: &str = "aws_datazone_policy_grant";
pub(super) fn resource(k: &str) -> bool {
    matches!(k, USER | GRANT)
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (USER | GRANT, "domain_identifier") => Some("DataZone domain"),
        (USER, "user_identifier") => Some("Typed DataZone user identity"),
        (GRANT, "entity_identifier") => Some("Domain-qualified grant entity"),
        (GRANT, "principal.project.project_identifier") => Some("DataZone project principal"),
        (GRANT, "principal.user.user_identifier") => Some("DataZone user principal"),
        (GRANT, "principal.group.group_identifier") => Some("External DataZone group principal"),
        (GRANT, "principal.domain_unit.domain_unit_identifier") => {
            Some("External DataZone domain unit")
        }
        _ => None,
    }
}
pub(super) fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (USER | GRANT, "domain_identifier", DOMAIN)
        | (GRANT, "entity_identifier", PROFILE)
        | (GRANT, "principal.project.project_identifier", PROJECT)
        | (GRANT, "principal.user.user_identifier", USER) => &["id"],
        (USER, "user_identifier", "aws_iam_user" | "aws_iam_role") => &["arn"],
        (GRANT, "entity_identifier", ASSET) => &["name"],
        (GRANT, "entity_identifier", CONFIGURATION) => &["environment_blueprint_id"],
        _ => &[],
    }
}
fn inactive(e: &[(&Value, &Value)], t: usize, p: &str) -> bool {
    value(e, t, p).is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn principal(e: &[(&Value, &Value)], t: usize, choice: &str) -> bool {
    let v = e[t].0;
    v["principal"].as_array().is_some_and(|a| a.len() == 1)
        && v["principal"][0][choice]
            .as_array()
            .is_some_and(|a| a.len() == 1 && a[0].is_object())
        && ["project", "user", "group", "domain_unit"]
            .into_iter()
            .all(|other| other == choice || inactive(e, t, &format!("principal.0.{other}")))
        && match choice {
            "project" => matches!(
                string(e, t, "principal.0.project.0.project_designation"),
                Some("OWNER" | "CONTRIBUTOR" | "PROJECT_CATALOG_STEWARD")
            ),
            "user" => inactive(e, t, "principal.0.user.0.all_users_grant_filter"),
            _ => false,
        }
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
            let p = r.attribute.as_str();
            label(k, p)?;
            let shape = if let Some(tail) = p.strip_prefix("principal.") {
                principal(e, r.target, tail.split('.').next().unwrap())
            } else {
                e[r.target].0[p].is_string()
            };
            let valid = |s: usize, _: &str| match (k, p) {
                (_, "domain_identifier") => true,
                (USER, "user_identifier") => matches!(
                    (
                        string(e, r.target, "user_type"),
                        nodes[s].resource_type.as_str()
                    ),
                    (Some("IAM_USER"), "aws_iam_user") | (Some("IAM_ROLE"), "aws_iam_role")
                ),
                (GRANT, "entity_identifier") => {
                    matches!(
                        (
                            string(e, r.target, "entity_type"),
                            nodes[s].resource_type.as_str()
                        ),
                        (Some("ASSET_TYPE"), ASSET)
                            | (Some("ENVIRONMENT_PROFILE"), PROFILE)
                            | (Some("ENVIRONMENT_BLUEPRINT_CONFIGURATION"), CONFIGURATION)
                    ) && same_domain(nodes, refs, e, r.target, s)
                }
                (GRANT, _) => same_domain(nodes, refs, e, r.target, s),
                _ => false,
            };
            Some(
                shape
                    && super::nested_references::verify(nodes, refs, e, r, fields, valid).is_some(),
            )
        })
        .collect();
    for (r, ok) in refs.iter_mut().zip(results) {
        if let Some(ok) = ok {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
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
        && (r.attribute == "domain_identifier"
            || (datazone::domain_bound(raw, r.target)
                && (k == USER || r.sources.iter().all(|&s| datazone::domain_bound(raw, s)))))
}
