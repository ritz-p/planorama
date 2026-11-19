use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_keyspaces_table", "keyspace_name") => Rule::new(
            "Keyspace",
            &[("aws_keyspaces_keyspace", &["name", "id"])],
            false,
            &[],
        ),
        ("aws_keyspaces_table", "encryption_specification.kms_key_identifier") => Rule::new(
            "Table encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_keyspaces_table")
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, _path: &str) -> bool {
    r.attribute != "encryption_specification.kms_key_identifier"
        || e.text(r.target, "encryption_specification.0.type") == Some("CUSTOMER_MANAGED_KMS_KEY")
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/keyspaces_configuration.rs"]
mod tests;
