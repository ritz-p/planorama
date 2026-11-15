use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/rds_auxiliary.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_rds_cluster_activity_stream"
            | "aws_rds_instance_state"
            | "aws_rds_shard_group"
            | "aws_rds_integration"
    )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_rds_cluster_activity_stream", "resource_arn") => Some("Activity stream cluster"),
        ("aws_rds_cluster_activity_stream" | "aws_rds_integration", "kms_key_id") => {
            Some("RDS auxiliary KMS key")
        }
        ("aws_rds_instance_state", "identifier") => Some("Managed DB instance"),
        ("aws_rds_shard_group", "db_cluster_identifier") => Some("Shard group cluster"),
        ("aws_rds_integration", "source_arn") => Some("Integration source database"),
        ("aws_rds_integration", "target_arn") => Some("Integration target warehouse"),
        _ => None,
    }
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_rds_cluster_activity_stream", "resource_arn", "aws_rds_cluster")
        | ("aws_rds_integration", "source_arn", "aws_rds_cluster" | "aws_db_instance")
        | ("aws_rds_integration", "target_arn", "aws_redshiftserverless_namespace") => &["arn"],
        ("aws_rds_integration", "target_arn", "aws_redshift_cluster") => &["cluster_namespace_arn"],
        ("aws_rds_instance_state", "identifier", "aws_db_instance") => &["identifier"],
        ("aws_rds_shard_group", "db_cluster_identifier", "aws_rds_cluster") => {
            &["cluster_identifier", "id"]
        }
        ("aws_rds_cluster_activity_stream", "kms_key_id", "aws_kms_alias") => {
            &["arn", "name", "id"]
        }
        (_, "kms_key_id", _) => super::kms::key_outputs(source),
        _ => &[],
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some(
                super::nested_references::verify(nodes, refs, e, r, fields, |source, path| {
                    let (v, m) = e[r.target];
                    super::nested_references::value(v, m, path).is_some_and(Value::is_string)
                        && (nodes[r.target].resource_type != "aws_rds_shard_group"
                            || super::service_evidence::known(
                                e,
                                source,
                                "cluster_scalability_type",
                            ) == Some("limitless"))
                })
                .is_some(),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let a = &raw.nodes[r.target];
        let b = &raw.nodes[s];
        let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        super::iam_policies::same_account(
            a.provider_configuration.as_ref(),
            b.provider_configuration.as_ref(),
        ) && ar.is_some()
            && ar == br
    })
}
