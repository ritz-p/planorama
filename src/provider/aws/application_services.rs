use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const AMPLIFY_APP: &str = "aws_amplify_app";
const AMPLIFY_BACKEND: &str = "aws_amplify_backend_environment";
const AMPLIFY_BRANCH: &str = "aws_amplify_branch";

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (AMPLIFY_APP, "iam_service_role_arn") => role("Amplify build service role"),
        (AMPLIFY_APP, "compute_role_arn") => role("Amplify SSR compute role"),
        (AMPLIFY_BACKEND | AMPLIFY_BRANCH, "app_id") => {
            scalar("Amplify app", &[(AMPLIFY_APP, &["id"])])
        }
        (AMPLIFY_BRANCH, "backend_environment_arn") => scalar(
            "Amplify backend environment",
            &[(AMPLIFY_BACKEND, &["arn"])],
        ),
        _ => return None,
    })
}

fn role(label: &'static str) -> Rule {
    scalar(label, &[("aws_iam_role", &["arn"])])
}

fn scalar(label: &'static str, outputs: &'static [(&str, &[&str])]) -> Rule {
    Rule::new(label, outputs, false, &[])
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
}

fn arn_resource<'a>(
    node: &TerraformEntity,
    arn: &'a str,
    service: &str,
    regional: bool,
) -> Option<&'a str> {
    let mut parts = arn.splitn(6, ':');
    if parts.next()? != "arn" || !parts.next()?.starts_with("aws") || parts.next()? != service {
        return None;
    }
    let region = parts.next()?;
    let account = parts.next()?;
    let resource = parts.next()?;
    let config = node.provider_configuration.as_ref()?;
    let valid_region = if regional {
        super::scope::scope(node)
            .and_then(|scope| scope.region)
            .is_some_and(|actual| actual.0 == region)
    } else {
        region.is_empty()
    };
    (valid_region
        && !config.account_conflict
        && crate::model::AwsAccountScope::new(account).is_some()
        && config
            .account
            .as_ref()
            .is_none_or(|actual| actual.as_str() == account)
        && !resource.is_empty())
    .then_some(resource)
}

fn endpoint(
    e: &Evidence<'_>,
    target: usize,
    path: &str,
    kind: &str,
    outputs: &[&str],
) -> Option<usize> {
    let r = super::lambda_core::unique(e.refs, target, path)?;
    let [source] = *r.sources.as_slice() else {
        return None;
    };
    let left = &e.nodes[target];
    let right = &e.nodes[source];
    let attrs = r.source_attributes_by_instance.get(&source)?;
    let output = attrs.first()?;
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == crate::model::EntityMode::Managed
            && node.action != crate::model::Action::Delete
            && node.deposed_key.is_none()
    };
    (r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && source != target
        && current(left)
        && current(right)
        && right.resource_type == kind
        && attrs.len() == 1
        && outputs.contains(&output.as_str())
        && e.text(target, path)
            .is_some_and(|value| e.text(source, output) == Some(value))
        && super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            super::scope::scope(left)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            super::scope::scope(right)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            false,
        ))
    .then_some(source)
}

fn same_endpoint(
    e: &Evidence<'_>,
    left: usize,
    right: usize,
    path: &str,
    kind: &str,
    outputs: &[&str],
) -> bool {
    endpoint(e, left, path, kind, outputs)
        .is_some_and(|owner| endpoint(e, right, path, kind, outputs) == Some(owner))
}

fn amplify_backend_in_app(e: &Evidence<'_>, backend: usize, app: usize) -> bool {
    let Some((id, environment)) = e.text(app, "id").zip(e.text(backend, "environment_name")) else {
        return false;
    };
    e.text(backend, "app_id") == Some(id)
        && e.text(backend, "arn")
            .and_then(|arn| arn_resource(&e.nodes[backend], arn, "amplify", true))
            .is_some_and(|resource| {
                resource == format!("apps/{id}/backendenvironments/{environment}")
            })
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let target = r.target;
    if e.nodes[source].resource_type == "aws_iam_role" {
        return e
            .text(source, "arn")
            .and_then(|arn| arn_resource(&e.nodes[source], arn, "iam", false))
            .and_then(|resource| resource.strip_prefix("role/"))
            .is_some_and(|name| !name.is_empty());
    }
    match (e.nodes[target].resource_type.as_str(), path) {
        (AMPLIFY_BACKEND, "app_id") => amplify_backend_in_app(e, target, source),
        (AMPLIFY_BRANCH, "backend_environment_arn") => {
            endpoint(e, target, "app_id", AMPLIFY_APP, &["id"]).is_some_and(|app| {
                same_endpoint(e, target, source, "app_id", AMPLIFY_APP, &["id"])
                    && amplify_backend_in_app(e, source, app)
            })
        }
        _ => true,
    }
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        AMPLIFY_BRANCH => Some(("app_id", "Amplify app", "Branches")),
        AMPLIFY_BACKEND => Some(("app_id", "Amplify app", "Backend environments")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/application_services.rs"]
mod tests;
