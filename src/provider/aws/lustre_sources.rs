use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_fsx_lustre_file_system", "backup_id") => Rule::new(
            "Lustre source backup",
            &[("aws_fsx_backup", &["id"])],
            false,
            &[],
        ),
        ("aws_fsx_lustre_file_system", "log_configuration.destination") => Rule::new(
            "Lustre log group",
            &[("aws_cloudwatch_log_group", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_fsx_lustre_file_system")
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
    match r.attribute.as_str() {
        "log_configuration.destination" => {
            matches!(
                e.text(r.target, "log_configuration.0.level"),
                Some("WARN_ONLY" | "FAILURE_ONLY" | "ERROR_ONLY" | "WARN_ERROR")
            ) && e
                .text(source, "name")
                .is_some_and(|name| name.starts_with("/aws/fsx"))
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/lustre_sources.rs"]
mod tests;
