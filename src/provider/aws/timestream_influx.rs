use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_timestreaminfluxdb_db_cluster" | "aws_timestreaminfluxdb_db_instance"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_timestreaminfluxdb_db_cluster", "vpc_subnet_ids") => {
            Rule::new("InfluxDB subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_timestreaminfluxdb_db_cluster", "vpc_security_group_ids") => Rule::new(
            "InfluxDB security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        (
            "aws_timestreaminfluxdb_db_cluster",
            "log_delivery_configuration.s3_configuration.bucket_name",
        ) => Rule::new(
            "Log delivery bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "log_delivery_configuration",
                "log_delivery_configuration.s3_configuration",
            ],
        ),
        ("aws_timestreaminfluxdb_db_instance", "vpc_subnet_ids") => {
            Rule::new("InfluxDB subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_timestreaminfluxdb_db_instance", "vpc_security_group_ids") => Rule::new(
            "InfluxDB security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        (
            "aws_timestreaminfluxdb_db_instance",
            "log_delivery_configuration.s3_configuration.bucket_name",
        ) => Rule::new(
            "Log delivery bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "log_delivery_configuration",
                "log_delivery_configuration.s3_configuration",
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _s: usize, _path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (_, "log_delivery_configuration.s3_configuration.bucket_name") => {
            e.value(
                r.target,
                "log_delivery_configuration.0.s3_configuration.0.enabled",
            )
            .and_then(Value::as_bool)
                == Some(true)
        }
        _ => true,
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_timestreaminfluxdb_db_cluster",
        ("vpc_subnet_ids", "vpc_security_group_ids"),
        bindings,
    );
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_timestreaminfluxdb_db_instance",
        ("vpc_subnet_ids", "vpc_security_group_ids"),
        bindings,
    );
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/timestream_influx.rs"]
mod tests;
