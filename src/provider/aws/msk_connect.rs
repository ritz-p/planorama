use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_mskconnect_connector" | "aws_mskconnect_custom_plugin"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_mskconnect_connector", "kafka_cluster.apache_kafka_cluster.vpc.subnets") => {
            Rule::new("Connector subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_mskconnect_connector", "kafka_cluster.apache_kafka_cluster.vpc.security_groups") => {
            Rule::new(
                "Connector security groups",
                &[("aws_security_group", &["id"])],
                true,
                &[],
            )
        }
        ("aws_mskconnect_connector", "service_execution_role_arn") => Rule::new(
            "Connector execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_mskconnect_connector", "plugin.custom_plugin.arn") => Rule::new(
            "Configured plugin revision",
            &[("aws_mskconnect_custom_plugin", &["arn"])],
            false,
            &["plugin"],
        ),
        ("aws_mskconnect_connector", "worker_configuration.arn") => Rule::new(
            "Worker configuration revision",
            &[("aws_mskconnect_worker_configuration", &["arn"])],
            false,
            &[],
        ),
        ("aws_mskconnect_custom_plugin", "location.s3.bucket_arn") => Rule::new(
            "Plugin source bucket",
            &[("aws_s3_bucket", &["arn"])],
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
    resource(kind)
        && (configured_links::indexed(kind, path, rule)
            || (kind == "aws_mskconnect_connector"
                && path.split('.').any(|p| p.parse::<usize>().is_ok())
                && matches!(
                    path.split('.')
                        .filter(|p| p.parse::<usize>().is_err())
                        .collect::<Vec<_>>()
                        .join(".")
                        .as_str(),
                    "plugin.custom_plugin.revision" | "worker_configuration.revision"
                )))
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
        ("aws_mskconnect_connector", "plugin.custom_plugin.arn" | "worker_configuration.arn") => {
            path.strip_suffix(".arn")
                .is_some_and(|b| revision(e, r.target, s, &format!("{b}.revision")))
        }
        _ => true,
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_mskconnect_connector",
        (
            "kafka_cluster.apache_kafka_cluster.vpc.subnets",
            "kafka_cluster.apache_kafka_cluster.vpc.security_groups",
        ),
        bindings,
    );
}
fn revision(e: &Evidence<'_>, i: usize, s: usize, path: &str) -> bool {
    super::lambda_core::unique(e.refs, i, path).is_some_and(|r| {
        r.complete
            && r.instances_explicit
            && r.issues.is_empty()
            && r.sources.as_slice() == [s]
            && r.source_attributes.len() == 1
            && r.source_attributes.contains("latest_revision")
            && e.value(i, path).and_then(Value::as_u64).is_some_and(|n| {
                n > 0 && e.value(s, "latest_revision").and_then(Value::as_u64) == Some(n)
            })
    })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/msk_connect.rs"]
mod tests;
