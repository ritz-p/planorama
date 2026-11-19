use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_dynamodb_table_export", "table_arn") => Rule::new(
            "Export source table",
            &[("aws_dynamodb_table", &["arn"])],
            false,
            &[],
        ),
        ("aws_dynamodb_table_export", "s3_bucket") => Rule::new(
            "Export bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_dynamodb_table_export", "s3_sse_kms_key_id") => Rule::new(
            "Export encryption key",
            &[("aws_kms_key", &["arn", "key_id", "id"])],
            false,
            &[],
        ),
        ("aws_dynamodb_table", "import_table.s3_bucket_source.bucket") => Rule::new(
            "Import source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_dynamodb_table", "restore_source_table_arn") => Rule::new(
            "Restore source table",
            &[("aws_dynamodb_table", &["arn"])],
            false,
            &[],
        ),
        ("aws_dynamodb_table", "restore_source_name") => Rule::new(
            "Restore source table",
            &[("aws_dynamodb_table", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_dynamodb_table_export" | "aws_dynamodb_table")
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
        "s3_sse_kms_key_id" => e.text(r.target, "s3_sse_algorithm") == Some("KMS"),
        "s3_bucket" | "import_table.s3_bucket_source.bucket" => {
            let field = if r.attribute == "s3_bucket" {
                "s3_bucket_owner"
            } else {
                "import_table.0.s3_bucket_source.0.bucket_owner"
            };
            e.value(r.target, field).is_some_and(|value| {
                value.is_null()
                    || value.as_str().is_some_and(|expected| {
                        e.nodes[source]
                            .provider_configuration
                            .as_ref()
                            .and_then(|config| config.account.as_ref())
                            .is_some_and(|actual| actual.as_str() == expected)
                    })
            })
        }
        "restore_source_table_arn" => e
            .value(r.target, "restore_source_name")
            .is_some_and(Value::is_null),
        "restore_source_name" => e
            .value(r.target, "restore_source_table_arn")
            .is_some_and(Value::is_null),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dynamodb_transfer.rs"]
mod tests;
