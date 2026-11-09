use crate::model::{AttributeReference, ProviderConfiguration, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/iam_policies.rs"]
mod tests;

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str)> {
    match kind {
        "aws_iam_group_policies_exclusive" | "aws_iam_group_policy_attachments_exclusive" => {
            Some(("group_name", "aws_iam_group"))
        }
        "aws_iam_user_policies_exclusive" | "aws_iam_user_policy_attachments_exclusive" => {
            Some(("user_name", "aws_iam_user"))
        }
        "aws_iam_role_policies_exclusive" | "aws_iam_role_policy_attachments_exclusive" => {
            Some(("role_name", "aws_iam_role"))
        }
        "aws_iam_role_policy" => Some(("role", "aws_iam_role")),
        _ => None,
    }
}

pub(super) fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, bool)> {
    if let Some((field, principal)) = owner(kind) {
        if path == field {
            return Some((principal, "name", false));
        }
        if kind.ends_with("_policy_attachments_exclusive") && path == "policy_arns" {
            return Some(("aws_iam_policy", "arn", true));
        }
        if kind.ends_with("_policies_exclusive") && path == "policy_names" {
            return Some((
                match principal {
                    "aws_iam_group" => "aws_iam_group_policy",
                    "aws_iam_user" => "aws_iam_user_policy",
                    _ => "aws_iam_role_policy",
                },
                "name",
                true,
            ));
        }
    }
    match (kind, path) {
        ("aws_iam_instance_profile" | "aws_iam_role_policy_attachment", "role") => {
            Some(("aws_iam_role", "name", false))
        }
        ("aws_iam_role_policy_attachment" | "aws_iam_policy_attachment", "policy_arn") => {
            Some(("aws_iam_policy", "arn", false))
        }
        ("aws_iam_policy_attachment", "groups") => Some(("aws_iam_group", "name", true)),
        ("aws_iam_policy_attachment", "users") => Some(("aws_iam_user", "name", true)),
        ("aws_iam_policy_attachment", "roles") => Some(("aws_iam_role", "name", true)),
        ("aws_iam_role" | "aws_iam_user", "permissions_boundary") => {
            Some(("aws_iam_policy", "arn", false))
        }
        ("aws_iam_role", "managed_policy_arns") => Some(("aws_iam_policy", "arn", true)),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path)?;
    Some(match path {
        "group_name" | "user_name" | "role_name" => "Policy controller owner",
        "role" => "Role",
        "permissions_boundary" => "Permissions boundary",
        "policy_names" => "Exclusive inline policies",
        "policy_arns" => "Exclusive managed policies",
        "managed_policy_arns" => "Managed policies",
        "groups" => "Attached groups",
        "users" => "Attached users",
        "roles" => "Attached roles",
        _ => "Direct policy",
    })
}

pub(super) fn same_account(
    left: Option<&ProviderConfiguration>,
    right: Option<&ProviderConfiguration>,
) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    !left.account_conflict
        && !right.account_conflict
        && match (&left.account, &right.account) {
            (Some(a), Some(b)) => a == b,
            (None, None) => left == right,
            _ => false,
        }
}

fn proof(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    super::iam::verified(nodes, r, evidence)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let valid = super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                && proof(nodes, r, evidence);
            if !valid || r.attribute != "policy_names" {
                return Some((valid, Vec::new()));
            }
            let (path, _) = owner(&nodes[r.target].resource_type)?;
            let controller = super::lambda_core::unique(refs, r.target, path)
                .filter(|p| proof(nodes, p, evidence));
            let context = controller
                .filter(|p| {
                    r.sources.iter().all(|&s| {
                        let parent_path = path.strip_suffix("_name").unwrap();
                        super::lambda_core::unique(refs, s, parent_path)
                            .is_some_and(|q| q.sources == p.sources && proof(nodes, q, evidence))
                    })
                })
                .map(|p| p.sources.clone())
                .unwrap_or_default();
            Some((!context.is_empty(), context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let compatible = |r: &AttributeReference| {
        r.sources.iter().all(|&s| {
            same_account(
                raw.nodes[r.target].provider_configuration.as_ref(),
                raw.nodes[s].provider_configuration.as_ref(),
            )
        })
    };
    if !compatible(r) {
        return false;
    }
    if r.attribute != "policy_names" {
        return true;
    }
    let Some((path, _)) = owner(&raw.nodes[r.target].resource_type) else {
        return false;
    };
    super::lambda_core::unique(&raw.attributes, r.target, path).is_some_and(compatible)
        && r.sources.iter().all(|&s| {
            super::lambda_core::unique(&raw.attributes, s, path.strip_suffix("_name").unwrap())
                .is_some_and(compatible)
        })
}
