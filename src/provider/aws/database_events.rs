use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/database_events.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_db_event_subscription", "sns_topic")
        | ("aws_docdb_event_subscription" | "aws_neptune_event_subscription", "sns_topic_arn") => {
            Some("Event topic")
        }
        (
            "aws_db_event_subscription"
            | "aws_docdb_event_subscription"
            | "aws_neptune_event_subscription",
            "source_ids",
        ) => Some("Event sources"),
        _ => None,
    }
}

fn output(service: &str, discriminator: &str, kind: &str, field: &str) -> bool {
    match (service, discriminator, kind) {
        (
            "aws_db_event_subscription",
            "db-instance",
            "aws_db_instance" | "aws_rds_cluster_instance",
        ) => field == "identifier",
        ("aws_docdb_event_subscription", "db-instance", "aws_docdb_cluster_instance")
        | ("aws_neptune_event_subscription", "db-instance", "aws_neptune_cluster_instance") => {
            matches!(field, "identifier" | "id")
        }
        ("aws_db_event_subscription", "db-cluster", "aws_rds_cluster")
        | ("aws_docdb_event_subscription", "db-cluster", "aws_docdb_cluster")
        | ("aws_neptune_event_subscription", "db-cluster", "aws_neptune_cluster") => {
            matches!(field, "cluster_identifier" | "id")
        }
        ("aws_db_event_subscription", "db-parameter-group", "aws_db_parameter_group")
        | (
            "aws_docdb_event_subscription",
            "db-parameter-group",
            "aws_docdb_cluster_parameter_group",
        )
        | ("aws_neptune_event_subscription", "db-parameter-group", "aws_neptune_parameter_group") =>
        {
            matches!(field, "name" | "id")
        }
        ("aws_db_event_subscription", "db-snapshot", "aws_db_snapshot") => {
            matches!(field, "db_snapshot_identifier" | "id")
        }
        ("aws_db_event_subscription", "db-snapshot", "aws_db_snapshot_copy") => {
            matches!(field, "target_db_snapshot_identifier" | "id")
        }
        ("aws_db_event_subscription", "db-cluster-snapshot", "aws_db_cluster_snapshot")
        | ("aws_docdb_event_subscription", "db-cluster-snapshot", "aws_docdb_cluster_snapshot")
        | (
            "aws_neptune_event_subscription",
            "db-cluster-snapshot",
            "aws_neptune_cluster_snapshot",
        ) => matches!(field, "db_cluster_snapshot_identifier" | "id"),
        ("aws_db_event_subscription", "db-cluster-snapshot", "aws_rds_cluster_snapshot_copy") => {
            matches!(field, "target_db_cluster_snapshot_identifier" | "id")
        }
        _ => false,
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        let collection = r.attribute == "source_ids";
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let field = r.source_attributes.iter().next().unwrap();
        let (value, mask) = evidence[r.target];
        let discriminator = super::service_references::scalar(value, mask, &["source_type"]);
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                let endpoint = &nodes[source];
                if source == r.target
                    || !current(endpoint)
                    || if collection {
                        !discriminator.is_some_and(|d| {
                            output(&node.resource_type, d, &endpoint.resource_type, field)
                        })
                    } else {
                        endpoint.resource_type != "aws_sns_topic" || field != "arn"
                    }
                {
                    return None;
                }
                let (value, mask) = evidence[source];
                super::service_references::scalar(value, mask, &[field])
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}
