use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/identitystore.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if kind != "aws_identitystore_group_membership" {
        return None;
    }
    match path {
        "group_id" => Some("Identity Store group"),
        "member_id" => Some("Identity Store member"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::service_evidence::annotate_scalar(
        nodes,
        refs,
        evidence,
        label,
        |_, path, kind, output| {
            matches!(
                (path, kind, output),
                ("group_id", "aws_identitystore_group", "group_id")
                    | ("member_id", "aws_identitystore_user", "user_id")
            )
        },
    );
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none()
            || !r.service_reference_verified
        {
            continue;
        }
        r.service_reference_verified &=
            super::service_evidence::known(evidence, r.target, "identity_store_id").is_some_and(
                |store| {
                    super::service_evidence::known(evidence, r.sources[0], "identity_store_id")
                        == Some(store)
                },
            );
    }
}
