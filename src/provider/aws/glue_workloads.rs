use crate::model::{AttributeReference, AwsAccountScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/glue_workloads.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_glue_crawler" | "aws_glue_job" | "aws_glue_ml_transform" | "aws_glue_trigger"
    )
}
fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (kind, path) {
        ("aws_glue_crawler", "classifiers") => {
            Some(("aws_glue_classifier", "name", "Crawler classifiers"))
        }
        ("aws_glue_crawler" | "aws_glue_job", "security_configuration")
        | ("aws_glue_trigger", "actions.security_configuration") => Some((
            "aws_glue_security_configuration",
            "name",
            "Security configuration",
        )),
        ("aws_glue_job", "connections")
        | (
            "aws_glue_crawler",
            "jdbc_target.connection_name"
            | "mongodb_target.connection_name"
            | "s3_target.connection_name"
            | "catalog_target.connection_name"
            | "delta_target.connection_name"
            | "hudi_target.connection_name"
            | "iceberg_target.connection_name",
        )
        | ("aws_glue_ml_transform", "input_record_tables.connection_name") => {
            Some(("aws_glue_connection", "name", "Glue connections"))
        }
        (
            "aws_glue_crawler",
            "s3_target.event_queue_arn"
            | "s3_target.dlq_event_queue_arn"
            | "catalog_target.event_queue_arn"
            | "catalog_target.dlq_event_queue_arn",
        ) => Some(("aws_sqs_queue", "arn", "Crawler event queue")),
        ("aws_glue_crawler", "dynamodb_target.path") => {
            Some(("aws_dynamodb_table", "name", "Crawler DynamoDB table"))
        }
        ("aws_glue_ml_transform", "role_arn") => {
            Some(("aws_iam_role", "arn", "Transform execution role"))
        }
        ("aws_glue_ml_transform", "input_record_tables.table_name") => {
            Some(("aws_glue_catalog_table", "name", "Transform input table"))
        }
        ("aws_glue_ml_transform", "input_record_tables.database_name") => Some((
            "aws_glue_catalog_database",
            "name",
            "Transform input database",
        )),
        ("aws_glue_trigger", "workflow_name") => {
            Some(("aws_glue_workflow", "name", "Trigger workflow"))
        }
        ("aws_glue_trigger", "actions.job_name" | "predicate.conditions.job_name") => {
            Some(("aws_glue_job", "name", "Configured job"))
        }
        ("aws_glue_trigger", "actions.crawler_name" | "predicate.conditions.crawler_name") => {
            Some(("aws_glue_crawler", "name", "Configured crawler"))
        }
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    let Some((expected, output, _)) = rule(kind, path) else {
        return &[];
    };
    if source != expected {
        return &[];
    }
    if source == "aws_glue_classifier" {
        return &["name", "id"];
    }
    match output {
        "name" => &["name"],
        "arn" => &["arn"],
        _ => &[],
    }
}
fn shape(value: &Value, path: &[&str]) -> bool {
    let Some((name, rest)) = path.split_first() else {
        return true;
    };
    let value = &value[*name];
    if rest.is_empty() {
        return value.is_null()
            || if matches!(*name, "classifiers" | "connections") {
                value.is_array()
            } else {
                value.is_string()
            };
    }
    value.as_array().is_some_and(|a| {
        !a.is_empty() && (*name != "predicate" || a.len() == 1) && a.iter().all(|v| shape(v, rest))
    })
}
fn value<'a>(evidence: &[(&'a Value, &Value)], index: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(evidence[index].0, evidence[index].1, path)
}
fn string<'a>(evidence: &[(&'a Value, &Value)], index: usize, path: &str) -> Option<&'a str> {
    value(evidence, index, path)?
        .as_str()
        .filter(|s| !s.is_empty())
}
fn compatible(nodes: &[TerraformEntity], target: usize, source: usize, global: bool) -> bool {
    let l = super::scope::scope(&nodes[target]);
    let r = super::scope::scope(&nodes[source]);
    (global
        || (l.as_ref().and_then(|s| s.region.as_ref()).is_some()
            && r.as_ref().and_then(|s| s.region.as_ref()).is_some()))
        && super::eventbridge::bindings(
            nodes[target].provider_configuration.as_ref(),
            nodes[source].provider_configuration.as_ref(),
            l.as_ref().and_then(|s| s.region.as_ref()),
            r.as_ref().and_then(|s| s.region.as_ref()),
            global,
        )
}
fn qualified(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let kind = nodes[r.target].resource_type.as_str();
    let Some((endpoint, _, _)) = rule(kind, &r.attribute) else {
        return false;
    };
    if !compatible(nodes, r.target, source, endpoint == "aws_iam_role") {
        return false;
    }
    if kind == "aws_glue_ml_transform" && path.starts_with("input_record_tables.") {
        let prefix = path.rsplit_once('.').unwrap().0;
        if string(evidence, r.target, &format!("{prefix}.catalog_id"))
            .is_none_or(|c| string(evidence, source, "catalog_id") != Some(c))
        {
            return false;
        }
        if endpoint == "aws_glue_catalog_table"
            && string(evidence, r.target, &format!("{prefix}.database_name"))
                .is_none_or(|d| string(evidence, source, "database_name") != Some(d))
        {
            return false;
        }
    }
    if kind == "aws_glue_trigger" {
        let mode = string(evidence, r.target, "type");
        if !matches!(
            mode,
            Some("ON_DEMAND" | "SCHEDULED" | "CONDITIONAL" | "EVENT")
        ) {
            return false;
        }
        if path.starts_with("predicate.") && mode != Some("CONDITIONAL") {
            return false;
        }
        if let Some((prefix, leaf)) = path.rsplit_once('.') {
            let counterpart = if leaf == "crawler_name" {
                "job_name"
            } else {
                "crawler_name"
            };
            if !value(evidence, r.target, &format!("{prefix}.{counterpart}"))
                .is_some_and(Value::is_null)
            {
                return false;
            }
            if leaf == "security_configuration"
                && string(evidence, r.target, &format!("{prefix}.job_name")).is_none()
            {
                return false;
            }
        }
    }
    true
}
fn needs_catalog_account(kind: &str, path: &str) -> bool {
    kind != "aws_glue_ml_transform"
        && rule(kind, path).is_some_and(|r| r.0 == "aws_glue_connection")
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = nodes[r.target].resource_type.as_str();
            label(kind, &r.attribute)?;
            let account = if needs_catalog_account(kind, &r.attribute) {
                let accounts = r
                    .sources
                    .iter()
                    .map(|&s| AwsAccountScope::new(string(evidence, s, "catalog_id")?))
                    .collect::<Option<BTreeSet<_>>>();
                accounts
                    .filter(|a| a.len() == 1)
                    .and_then(|a| a.into_iter().next())
            } else {
                None
            };
            let valid = (!needs_catalog_account(kind, &r.attribute) || account.is_some())
                && shape(
                    evidence[r.target].0,
                    &r.attribute.split('.').collect::<Vec<_>>(),
                )
                && super::nested_references::verify(
                    nodes,
                    refs,
                    evidence,
                    r,
                    fields,
                    |source, path| qualified(nodes, evidence, r, source, path),
                )
                .is_some();
            Some((valid, account))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, account)) = result {
            r.service_reference_verified = valid;
            r.service_reference_owner_account = account;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let kind = raw.nodes[r.target].resource_type.as_str();
    let Some((endpoint, _, _)) = rule(kind, &r.attribute) else {
        return true;
    };
    if needs_catalog_account(kind, &r.attribute) {
        let account = raw.nodes[r.target]
            .provider_configuration
            .as_ref()
            .and_then(|b| b.account.as_ref());
        if account.is_none() || account != r.service_reference_owner_account.as_ref() {
            return false;
        }
    }
    r.sources.iter().all(|&s| {
        let l = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint == "aws_iam_role",
        )
    })
}
