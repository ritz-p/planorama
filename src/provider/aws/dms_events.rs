use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
const EVENT: &str = "aws_dms_event_subscription";
const INSTANCE: &str = "aws_dms_replication_instance";
const TASK: &str = "aws_dms_replication_task";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, EVENT | INSTANCE)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (EVENT, "sns_topic_arn") => Some("DMS event topic"),
        (EVENT, "source_ids") => Some("DMS event sources"),
        (INSTANCE, "kms_key_arn") => Some("DMS instance encryption key"),
        (INSTANCE, "kerberos_authentication_settings.key_cache_secret_iam_arn") => {
            Some("DMS Kerberos secret role")
        }
        (INSTANCE, "kerberos_authentication_settings.key_cache_secret_id") => {
            Some("DMS Kerberos secret")
        }
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == INSTANCE
        && matches!(
            path,
            "kerberos_authentication_settings.0.key_cache_secret_iam_arn"
                | "kerberos_authentication_settings.0.key_cache_secret_id"
        )
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("sns_topic_arn", "aws_sns_topic")
        | ("kms_key_arn", "aws_kms_key")
        | ("kerberos_authentication_settings.key_cache_secret_iam_arn", "aws_iam_role") => &["arn"],
        ("kerberos_authentication_settings.key_cache_secret_id", "aws_secretsmanager_secret") => {
            &["arn", "id", "name"]
        }
        ("source_ids", INSTANCE) => &["replication_instance_id"],
        ("source_ids", TASK) => &["replication_task_id"],
        _ => &[],
    }
}
fn compatible(l: &TerraformEntity, r: &TerraformEntity) -> bool {
    let left = super::scope::scope(l);
    let right = super::scope::scope(r);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        left.as_ref().and_then(|s| s.region.as_ref()),
        right.as_ref().and_then(|s| s.region.as_ref()),
        r.resource_type == "aws_iam_role",
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let path = r.attribute.replace(
                "kerberos_authentication_settings.",
                "kerberos_authentication_settings.0.",
            );
            let (v, m) = e[r.target];
            let shape = super::nested_references::value(v, m, &path).is_some_and(|value| {
                if r.attribute == "source_ids" {
                    value.as_array().is_some_and(|a| !a.is_empty())
                } else {
                    value.as_str().is_some_and(|s| !s.is_empty())
                }
            });
            Some(
                shape
                    && super::nested_references::verify(nodes, refs, e, r, fields, |s, _| {
                        compatible(&nodes[r.target], &nodes[s])
                            && (r.attribute != "source_ids"
                                || matches!(
                                    (
                                        super::service_evidence::known(e, r.target, "source_type"),
                                        nodes[s].resource_type.as_str()
                                    ),
                                    (Some("replication-instance"), INSTANCE)
                                        | (Some("replication-task"), TASK)
                                ))
                    })
                    .is_some(),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
            let l = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.resource_type == "aws_iam_role",
            )
        })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dms_events.rs"]
mod tests;
