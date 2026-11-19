use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_qbusiness_application", "iam_service_role_arn") => Rule::new(
            "Q Business service role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_qbusiness_application", "identity_center_instance_arn") => Rule::new(
            "Identity Center instance",
            &[("aws_ssoadmin_instances", &["arns"])],
            false,
            &[],
        ),
        ("aws_qbusiness_application", "encryption_configuration.kms_key_id") => Rule::new(
            "Q Business encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id", "id"]),
                ("aws_kms_replica_key", &["arn", "key_id", "id"]),
                ("aws_kms_external_key", &["arn", "id"]),
            ],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_qbusiness_application")
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    if r.attribute != "encryption_configuration.kms_key_id" {
        return true;
    }
    let (key, mask) = e.values[source];
    super::kms::symmetric_key(&e.nodes[source].resource_type, key, mask)
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/qbusiness.rs"]
mod tests;
