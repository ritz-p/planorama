use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/prefix_list_entries.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    (kind == "aws_ec2_managed_prefix_list_entry" && path == "prefix_list_id")
        .then_some("Prefix list owner")
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
        |_, _, source, field| source == "aws_ec2_managed_prefix_list" && field == "id",
    );
}
