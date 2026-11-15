use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transfer_identity.rs"]
mod tests;

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        "aws_transfer_access" => Some(("server_id", "Transfer server", "Access grants")),
        "aws_transfer_user" => Some(("server_id", "Transfer server", "Users")),
        "aws_transfer_host_key" => Some(("server_id", "Transfer server", "Host keys")),
        "aws_transfer_ssh_key" => Some(("user_name", "Transfer user", "SSH keys")),
        "aws_transfer_tag" => Some(("resource_arn", "Transfer tag owner", "Tags")),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (
            "aws_transfer_access"
            | "aws_transfer_user"
            | "aws_transfer_host_key"
            | "aws_transfer_ssh_key",
            "server_id",
        ) => Some("Transfer server"),
        ("aws_transfer_access" | "aws_transfer_user", "role") => Some("Transfer identity role"),
        ("aws_transfer_ssh_key", "user_name") => Some("Transfer user"),
        ("aws_transfer_tag", "resource_arn") => Some("Transfer tag owner"),
        _ => None,
    }
}
pub(super) fn taggable(kind: &str) -> bool {
    matches!(
        kind,
        "aws_transfer_server"
            | "aws_transfer_user"
            | "aws_transfer_host_key"
            | "aws_transfer_certificate"
            | "aws_transfer_profile"
            | "aws_transfer_agreement"
            | "aws_transfer_connector"
            | "aws_transfer_web_app"
            | "aws_transfer_workflow"
    )
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    if label(kind, path).is_none() {
        return &[];
    }
    match (path, source) {
        ("server_id", "aws_transfer_server") => &["id"],
        ("role", "aws_iam_role") => &["arn"],
        ("user_name", "aws_transfer_user") => &["user_name"],
        ("resource_arn", kind) if taggable(kind) => &["arn"],
        _ => &[],
    }
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some(
                evidence[r.target].0[&r.attribute].is_string()
                    && super::nested_references::verify(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |_, _| true,
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let compatible = r.sources.iter().all(|&s| {
        let l = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.resource_type == "aws_iam_role",
        )
    });
    if !compatible {
        return false;
    }
    if r.attribute != "user_name" {
        return true;
    }
    let [user] = r.sources.as_slice() else {
        return false;
    };
    let server = |t| {
        super::lambda_core::unique(&raw.attributes, t, "server_id")
            .filter(|r| r.service_reference_verified && bindings(raw, r))
    };
    server(r.target)
        .zip(server(*user))
        .is_some_and(|(key, user)| key.sources.len() == 1 && key.sources == user.sources)
}
