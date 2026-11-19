use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_athena_data_catalog"
            | "aws_athena_database"
            | "aws_athena_named_query"
            | "aws_athena_prepared_statement"
            | "aws_athena_workgroup"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_athena_data_catalog", "parameters") => Rule::new(
            "Catalog Lambda functions",
            &[("aws_lambda_function", &["arn"])],
            true,
            &[],
        )
        .decoded(decode),
        ("aws_athena_database", "bucket") => Rule::new(
            "Database result bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_athena_database", "workgroup") => Rule::new(
            "Database workgroup",
            &[("aws_athena_workgroup", &["name"])],
            false,
            &[],
        ),
        ("aws_athena_database", "encryption_configuration.kms_key") => Rule::new(
            "Database encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_athena_named_query", "workgroup") => Rule::new(
            "Query workgroup",
            &[("aws_athena_workgroup", &["name"])],
            false,
            &[],
        ),
        ("aws_athena_prepared_statement", "workgroup") => Rule::new(
            "Query workgroup",
            &[("aws_athena_workgroup", &["name"])],
            false,
            &[],
        ),
        ("aws_athena_named_query", "database") => Rule::new(
            "Athena database",
            &[("aws_athena_database", &["name"])],
            false,
            &[],
        ),
        ("aws_athena_workgroup", "configuration.execution_role") => Rule::new(
            "Workgroup execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_athena_workgroup",
            "configuration.identity_center_configuration.identity_center_instance_arn",
        ) => Rule::new(
            "Identity Center instance",
            &[("aws_ssoadmin_instances", &["arns"])],
            false,
            &[],
        ),
        (
            "aws_athena_workgroup",
            "configuration.customer_content_encryption_configuration.kms_key",
        ) => Rule::new(
            "Customer content key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_athena_workgroup",
            "configuration.managed_query_results_configuration.encryption_configuration.kms_key",
        ) => Rule::new(
            "Managed results key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_athena_workgroup",
            "configuration.result_configuration.encryption_configuration.kms_key_arn",
        ) => Rule::new("Query output key", &[("aws_kms_key", &["arn"])], false, &[]),
        ("aws_athena_workgroup", "configuration.result_configuration.output_location") => {
            Rule::new(
                "Query result bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
            .decoded(decode)
        }
        (
            "aws_athena_workgroup",
            "configuration.monitoring_configuration.s3_logging_configuration.log_location",
        ) => Rule::new(
            "Workgroup logging bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .decoded(decode),
        (
            "aws_athena_workgroup",
            "configuration.monitoring_configuration.s3_logging_configuration.kms_key",
        ) => Rule::new("S3 logging key", &[("aws_kms_key", &["arn"])], false, &[]),
        (
            "aws_athena_workgroup",
            "configuration.monitoring_configuration.managed_logging_configuration.kms_key",
        ) => Rule::new(
            "Managed logging key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_athena_workgroup",
            "configuration.monitoring_configuration.cloud_watch_logging_configuration.log_group",
        ) => Rule::new(
            "Workgroup log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[],
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
        ("aws_athena_data_catalog", "parameters") => {
            let Some(m) = e.value(r.target, "parameters").and_then(Value::as_object) else {
                return false;
            };
            match e.text(r.target, "type") {
                Some("LAMBDA") => {
                    (m.len() == 1 && m.contains_key("function"))
                        || (m.len() == 2
                            && m.contains_key("metadata-function")
                            && m.contains_key("record-function"))
                }
                Some("HIVE") => m.len() == 1 && m.contains_key("metadata-function"),
                _ => false,
            }
        }
        ("aws_athena_database", "bucket") => owner(e, r.target, s, "expected_bucket_owner"),
        ("aws_athena_database", "encryption_configuration.kms_key") => matches!(
            e.text(r.target, "encryption_configuration.0.encryption_option"),
            Some("SSE_KMS" | "CSE_KMS")
        ),
        ("aws_athena_workgroup", p) if p.starts_with("configuration.result_configuration.") => {
            let enabled = e.value(
                r.target,
                "configuration.0.managed_query_results_configuration.0.enabled",
            );
            (empty(
                e,
                r.target,
                "configuration.0.managed_query_results_configuration",
            ) || enabled.and_then(Value::as_bool) == Some(false))
                && if p.ends_with(".output_location") {
                    owner(
                        e,
                        r.target,
                        s,
                        "configuration.0.result_configuration.0.expected_bucket_owner",
                    )
                } else {
                    matches!(e.text(r.target,"configuration.0.result_configuration.0.encryption_configuration.0.encryption_option"),Some("SSE_KMS"|"CSE_KMS"))
                }
        }
        ("aws_athena_workgroup", p)
            if p.starts_with("configuration.managed_query_results_configuration.") =>
        {
            e.value(
                r.target,
                "configuration.0.managed_query_results_configuration.0.enabled",
            )
            .and_then(Value::as_bool)
                == Some(true)
                && empty(e, r.target, "configuration.0.result_configuration")
        }
        ("aws_athena_workgroup", p) if p.starts_with("configuration.monitoring_configuration.") => {
            path.rsplit_once('.').is_some_and(|(b, _)| {
                e.value(r.target, &format!("{b}.enabled"))
                    .and_then(Value::as_bool)
                    == Some(true)
            })
        }
        ("aws_athena_workgroup", p) if p.ends_with("identity_center_instance_arn") => {
            e.value(
                r.target,
                "configuration.0.identity_center_configuration.0.enable_identity_center",
            )
            .and_then(Value::as_bool)
                == Some(true)
        }
        _ => true,
    }
}
fn empty(e: &Evidence<'_>, i: usize, p: &str) -> bool {
    e.value(i, p)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn owner(e: &Evidence<'_>, i: usize, s: usize, p: &str) -> bool {
    e.value(i, p).is_some_and(|v| {
        v.is_null()
            || v.as_str().is_some_and(|id| {
                e.nodes[s]
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
                    .is_some_and(|a| a.as_str() == id)
            })
    })
}
pub(super) fn decode<'a>(p: &str, v: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    if p == "parameters" {
        let values = v.as_object()?;
        return values.values().map(Value::as_str).collect();
    }
    let uri = v.as_str()?.strip_prefix("s3://")?;
    let b = uri.split('/').next()?;
    (!b.is_empty() && !b.contains(['@', ':', '?', '#'])).then_some([b].into())
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/athena_queries.rs"]
mod tests;
