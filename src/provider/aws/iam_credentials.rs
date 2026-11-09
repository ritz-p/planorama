use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/iam_credentials.rs"]
mod tests;

pub(super) fn owner(kind: &str) -> Option<&'static str> {
    match kind {
        "aws_iam_access_key" | "aws_iam_user_login_profile" => Some("user"),
        "aws_iam_service_specific_credential" | "aws_iam_signing_certificate" => Some("user_name"),
        "aws_iam_user_ssh_key" => Some("username"),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    (owner(kind) == Some(path)).then_some("Credential owner")
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let valid = super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                && current(&nodes[r.target])
                && r.complete
                && r.instances_explicit
                && r.issues.is_empty()
                && r.sources.len() == 1
                && r.source_attributes.len() == 1
                && r.source_attributes.contains("name")
                && r.sources.iter().all(|&s| {
                    current(&nodes[s])
                        && nodes[s].resource_type == "aws_iam_user"
                        && super::service_evidence::known(evidence, r.target, &r.attribute)
                            .is_some_and(|v| {
                                super::service_evidence::known(evidence, s, "name") == Some(v)
                            })
                });
            Some(valid)
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&s| {
        super::iam_policies::same_account(
            raw.nodes[r.target].provider_configuration.as_ref(),
            raw.nodes[s].provider_configuration.as_ref(),
        )
    })
}
