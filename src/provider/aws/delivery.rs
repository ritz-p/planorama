use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const IAM_OWNER: &[(&str, &[&str])] = &[("aws_iam_user", &["arn"]), ("aws_iam_role", &["arn"])];

fn rule(kind: &str, path: &str) -> Option<Rule> {
    if let Some(rule) = codecatalyst::rule(kind, path) {
        return Some(rule);
    }
    if let Some(rule) = codebuild::rule(kind, path) {
        return Some(rule);
    }
    if let Some(rule) = codeartifact_rule(kind, path) {
        return Some(rule);
    }
    let (label, outputs): (_, &'static [(&str, &[&str])]) = match (kind, path) {
        ("aws_cloud9_environment_ec2", "subnet_id") => {
            ("Cloud9 subnet", &[("aws_subnet", &["id"])])
        }
        ("aws_cloud9_environment_ec2", "owner_arn") => ("Cloud9 owner", IAM_OWNER),
        ("aws_cloud9_environment_membership", "environment_id") => (
            "Cloud9 environment",
            &[("aws_cloud9_environment_ec2", &["id"])],
        ),
        ("aws_cloud9_environment_membership", "user_arn") => {
            ("Cloud9 member", &[("aws_iam_user", &["arn"])])
        }
        _ => return None,
    };
    Some(Rule::new(label, outputs, false, &[]))
}

pub(crate) fn resource(kind: &str) -> bool {
    if codecatalyst::resource(kind) {
        return true;
    }
    if codebuild::resource(kind) {
        return true;
    }
    matches!(
        kind,
        "aws_codeartifact_domain"
            | "aws_codeartifact_domain_permissions_policy"
            | "aws_codeartifact_repository"
            | "aws_codeartifact_repository_permissions_policy"
    ) || matches!(
        kind,
        "aws_cloud9_environment_ec2" | "aws_cloud9_environment_membership"
    )
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}

fn qualifies(e: &Evidence<'_>, reference: &AttributeReference, source: usize, path: &str) -> bool {
    let kind = &e.nodes[reference.target].resource_type;
    if codecatalyst::resource(kind) {
        return codecatalyst::qualifies(e, reference, source, path);
    }
    if codebuild::resource(kind) {
        return codebuild::qualifies(e, reference, source, path);
    }
    if kind.starts_with("aws_codeartifact_") && reference.attribute != "encryption_key" {
        return codeartifact_qualifies(e, reference, source);
    }
    true
}

fn codeartifact_rule(kind: &str, path: &str) -> Option<Rule> {
    let (label, outputs, blocks): (_, &'static [(&str, &[&str])], &'static [&str]) =
        match (kind, path) {
            ("aws_codeartifact_domain", "encryption_key") => {
                return Some(Rule::new(
                    "CodeArtifact encryption key",
                    &[("aws_kms_key", &["arn"])],
                    false,
                    &[],
                ));
            }
            (
                "aws_codeartifact_repository" | "aws_codeartifact_domain_permissions_policy",
                "domain",
            ) => (
                "CodeArtifact domain",
                &[("aws_codeartifact_domain", &["domain"])],
                &[],
            ),
            ("aws_codeartifact_repository", "upstream.repository_name") => (
                "CodeArtifact upstream repository",
                &[("aws_codeartifact_repository", &["repository"])],
                &["upstream"],
            ),
            ("aws_codeartifact_repository_permissions_policy", "repository") => (
                "CodeArtifact repository",
                &[("aws_codeartifact_repository", &["repository"])],
                &[],
            ),
            _ => return None,
        };
    Some(Rule::new(label, outputs, false, blocks).with_explicit_destination())
}

fn codeartifact_qualifies(e: &Evidence<'_>, reference: &AttributeReference, source: usize) -> bool {
    use super::configured_links::same_account;
    use super::scope::scope;
    let consumer = reference.target;
    let left = &e.nodes[consumer];
    let right = &e.nodes[source];
    let region = |node: &TerraformEntity| scope(node).and_then(|s| s.region);
    if !region(left).zip(region(right)).is_some_and(|(a, b)| a == b) {
        return false;
    }
    let account = |node: usize| {
        e.nodes[node]
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            .map(|a| a.as_str())
    };
    let owner = |node: usize| {
        let field = if e.nodes[node].resource_type == "aws_codeartifact_domain" {
            "owner"
        } else {
            "domain_owner"
        };
        if e.absent(node, field) {
            return Some(account(node));
        }
        let value = e.text(node, field)?;
        crate::model::AwsAccountScope::new(value)?;
        if field == "owner" && account(node).is_some_and(|actual| actual != value) {
            return None;
        }
        Some(Some(value))
    };
    let same_owner = match (owner(consumer), owner(source)) {
        (Some(Some(left)), Some(Some(right))) => left == right,
        (Some(None), Some(None)) => same_account(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
        ),
        _ => false,
    };
    same_owner
        && e.text(consumer, "domain")
            .zip(e.text(source, "domain"))
            .is_some_and(|(a, b)| a == b)
}

mod codebuild;

mod codecatalyst;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/delivery.rs"]
mod tests;
