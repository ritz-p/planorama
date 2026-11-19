use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_timestreamwrite_database"
            | "aws_timestreamwrite_table"
            | "aws_timestreamquery_scheduled_query"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_timestreamwrite_database", "kms_key_id") => Rule::new(
            "Database encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_timestreamwrite_table", "database_name") => Rule::new(
            "Timestream database",
            &[("aws_timestreamwrite_database", &["database_name", "id"])],
            false,
            &[],
        ),
        (
            "aws_timestreamwrite_table",
            "magnetic_store_write_properties.magnetic_store_rejected_data_location.s3_configuration.bucket_name",
        ) => Rule::new(
            "Rejected data bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        (
            "aws_timestreamwrite_table",
            "magnetic_store_write_properties.magnetic_store_rejected_data_location.s3_configuration.kms_key_id",
        ) => Rule::new(
            "Rejected data key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_timestreamquery_scheduled_query",
            "target_configuration.timestream_configuration.database_name",
        ) => Rule::new(
            "Target database",
            &[("aws_timestreamwrite_database", &["database_name", "id"])],
            false,
            &[
                "target_configuration",
                "target_configuration.timestream_configuration",
            ],
        ),
        (
            "aws_timestreamquery_scheduled_query",
            "target_configuration.timestream_configuration.table_name",
        ) => Rule::new(
            "Target table",
            &[("aws_timestreamwrite_table", &["table_name"])],
            false,
            &[
                "target_configuration",
                "target_configuration.timestream_configuration",
            ],
        ),
        ("aws_timestreamquery_scheduled_query", "execution_role_arn") => Rule::new(
            "Query execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_timestreamquery_scheduled_query", "kms_key_id") => Rule::new(
            "Query encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        (
            "aws_timestreamquery_scheduled_query",
            "notification_configuration.sns_configuration.topic_arn",
        ) => Rule::new(
            "Query notifications",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[
                "notification_configuration",
                "notification_configuration.sns_configuration",
            ],
        ),
        (
            "aws_timestreamquery_scheduled_query",
            "error_report_configuration.s3_configuration.bucket_name",
        ) => Rule::new(
            "Query error bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "error_report_configuration",
                "error_report_configuration.s3_configuration",
            ],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (
            "aws_timestreamquery_scheduled_query",
            "target_configuration.timestream_configuration.table_name",
        ) => {
            let fields = &["database_name", "id"];
            let kind = "aws_timestreamwrite_database";
            e.configured_endpoint(
                r.target,
                "target_configuration.0.timestream_configuration.0.database_name",
                kind,
                fields,
            )
            .zip(e.configured_endpoint(s, "database_name", kind, fields))
            .is_some_and(|(query_database, table_database)| query_database == table_database)
        }
        ("aws_timestreamwrite_table", p) if p.starts_with("magnetic_store_write_properties.") => {
            e.value(
                r.target,
                "magnetic_store_write_properties.0.enable_magnetic_store_writes",
            )
            .and_then(Value::as_bool)
                == Some(true)
                && (!p.ends_with(".kms_key_id")
                    || path.rsplit_once('.').is_some_and(|(b, _)| {
                        e.text(r.target, &format!("{b}.encryption_option")) == Some("SSE_KMS")
                    }))
        }
        _ => true,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/timestream_queries.rs"]
mod tests;
