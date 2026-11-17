use super::{nested_references, service_evidence};
use crate::model::{AttributeReference, AwsAccountScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const CLUSTER: &str = "aws_redshift_cluster";
const EVENT: &str = "aws_redshift_event_subscription";
const LOGGING: &str = "aws_redshift_logging";
const PARTNER: &str = "aws_redshift_partner";
const ACTION: &str = "aws_redshift_scheduled_action";
const USAGE: &str = "aws_redshift_usage_limit";
const STATEMENT: &str = "aws_redshiftdata_statement";
const WORKGROUP: &str = "aws_redshiftserverless_workgroup";
const ACTIONS: [&str; 3] = ["pause_cluster", "resize_cluster", "resume_cluster"];

pub(super) fn resource(kind: &str) -> bool {
    matches!(kind, EVENT | LOGGING | PARTNER | ACTION | USAGE | STATEMENT)
}
pub(crate) fn has_indexed(kind: &str) -> bool {
    kind == ACTION
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == ACTION
        && matches!(path.split('.').collect::<Vec<_>>().as_slice(),
        ["target_action", "0", action, "0", "cluster_identifier"] if ACTIONS.contains(action))
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        LOGGING => Some(("cluster_identifier", "Logging cluster", "Audit logging")),
        USAGE => Some(("cluster_identifier", "Usage limit cluster", "Usage limits")),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (LOGGING | USAGE, "cluster_identifier") => owner(kind).map(|(_, label, _)| label),
        (EVENT, "sns_topic_arn") => Some("Redshift event topic"),
        (EVENT, "source_ids") => Some("Redshift event sources"),
        (LOGGING, "bucket_name") => Some("Audit log bucket"),
        (PARTNER, "cluster_identifier") => Some("Partner cluster"),
        (ACTION, "iam_role") => Some("Scheduled action role"),
        (
            ACTION,
            "target_action.pause_cluster.cluster_identifier"
            | "target_action.resize_cluster.cluster_identifier"
            | "target_action.resume_cluster.cluster_identifier",
        ) => Some("Scheduled action cluster"),
        (STATEMENT, "cluster_identifier") => Some("Statement cluster"),
        (STATEMENT, "workgroup_name") => Some("Statement workgroup"),
        (STATEMENT, "secret_arn") => Some("Statement credentials secret"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("sns_topic_arn", "aws_sns_topic")
        | ("iam_role", "aws_iam_role")
        | ("secret_arn", "aws_secretsmanager_secret") => &["arn"],
        (
            "source_ids"
            | "cluster_identifier"
            | "target_action.pause_cluster.cluster_identifier"
            | "target_action.resize_cluster.cluster_identifier"
            | "target_action.resume_cluster.cluster_identifier",
            CLUSTER,
        ) => &["cluster_identifier"],
        ("source_ids", "aws_redshift_parameter_group" | ACTION) => &["name"],
        ("source_ids", "aws_redshift_cluster_snapshot") => &["snapshot_identifier"],
        ("bucket_name", "aws_s3_bucket") => &["bucket", "id"],
        ("workgroup_name", WORKGROUP) => &["workgroup_name"],
        _ => &[],
    }
}
fn absent_block(v: &Value, m: &Value, path: &str) -> bool {
    nested_references::value(v, m, path)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn valid_mode(kind: &str, path: &str, source: &str, v: &Value, m: &Value) -> bool {
    let known = |p| nested_references::value(v, m, p).and_then(Value::as_str);
    match (kind, path) {
        (EVENT, "source_ids") => matches!(
            (known("source_type"), source),
            (Some("cluster"), CLUSTER)
                | (
                    Some("cluster-parameter-group"),
                    "aws_redshift_parameter_group"
                )
                | (Some("cluster-snapshot"), "aws_redshift_cluster_snapshot")
                | (Some("scheduled-action"), ACTION)
        ),
        (LOGGING, "bucket_name") => known("log_destination_type") == Some("s3"),
        (STATEMENT, "cluster_identifier") => {
            nested_references::value(v, m, "workgroup_name").is_some_and(Value::is_null)
        }
        (STATEMENT, "workgroup_name") => {
            nested_references::value(v, m, "cluster_identifier").is_some_and(Value::is_null)
        }
        (ACTION, p) if p.starts_with("target_action.") => {
            let Some(branch) = p.split('.').nth(1) else {
                return false;
            };
            nested_references::singleton(v, m, "target_action")
                && ACTIONS.iter().all(|&b| {
                    let path = format!("target_action.0.{b}");
                    if b == branch {
                        nested_references::singleton(v, m, &path)
                    } else {
                        absent_block(v, m, &path)
                    }
                })
        }
        _ => true,
    }
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
            let path = r.attribute.as_str();
            label(kind, path)?;
            let (v, m) = evidence[r.target];
            let account = (kind == PARTNER)
                .then(|| {
                    service_evidence::known(evidence, r.target, "account_id")
                        .and_then(AwsAccountScope::new)
                })
                .flatten();
            let valid = (kind != PARTNER || account.is_some())
                && nested_references::verify_collections(
                    nodes,
                    refs,
                    evidence,
                    r,
                    fields,
                    |s, indexed| {
                        let shape = nested_references::value(v, m, indexed).is_some_and(|leaf| {
                            if path == "source_ids" {
                                leaf.is_array()
                            } else {
                                leaf.is_string()
                            }
                        });
                        shape && valid_mode(kind, path, &nodes[s].resource_type, v, m)
                    },
                    &[
                        "target_action",
                        "pause_cluster",
                        "resize_cluster",
                        "resume_cluster",
                    ],
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
    let consumer = &raw.nodes[r.target];
    if label(&consumer.resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        let binding = endpoint.provider_configuration.as_ref();
        super::eventbridge::bindings(
            consumer.provider_configuration.as_ref(),
            binding,
            consumer
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role",
        ) && (consumer.resource_type != PARTNER
            || r.service_reference_owner_account
                .as_ref()
                .is_some_and(|a| binding.and_then(|b| b.account.as_ref()) == Some(a)))
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/redshift_helpers.rs"]
mod tests;
