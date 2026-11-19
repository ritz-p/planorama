use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_finspace_kx_environment", "kms_key_id") => Rule::new(
            "Environment encryption key",
            &[("aws_kms_key", &["arn", "id", "key_id"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_environment", "transit_gateway_configuration.transit_gateway_id") => {
            Rule::new(
                "Environment transit gateway",
                &[("aws_ec2_transit_gateway", &["id"])],
                false,
                &[],
            )
        }
        (
            "aws_finspace_kx_cluster"
            | "aws_finspace_kx_database"
            | "aws_finspace_kx_dataview"
            | "aws_finspace_kx_scaling_group"
            | "aws_finspace_kx_user"
            | "aws_finspace_kx_volume",
            "environment_id",
        ) => Rule::new(
            "KX environment",
            &[("aws_finspace_kx_environment", &["id"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_cluster", "vpc_configuration.subnet_ids") => {
            Rule::new("Endpoint subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_finspace_kx_cluster", "vpc_configuration.security_group_ids") => Rule::new(
            "Endpoint security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_finspace_kx_cluster", "vpc_configuration.vpc_id") => {
            Rule::new("Endpoint VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_finspace_kx_cluster", "execution_role") => Rule::new(
            "Cluster execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_user", "iam_role") => Rule::new(
            "KX user IAM role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_cluster", "database.database_name") => Rule::new(
            "Cluster database",
            &[("aws_finspace_kx_database", &["name"])],
            false,
            &["database"],
        ),
        ("aws_finspace_kx_cluster", "database.dataview_name") => Rule::new(
            "Cluster dataview",
            &[("aws_finspace_kx_dataview", &["name"])],
            false,
            &["database"],
        ),
        ("aws_finspace_kx_dataview", "database_name") => Rule::new(
            "Dataview database",
            &[("aws_finspace_kx_database", &["name"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_cluster", "scaling_group_configuration.scaling_group_name") => Rule::new(
            "Cluster scaling group",
            &[("aws_finspace_kx_scaling_group", &["name"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_cluster", "savedown_storage_configuration.volume_name") => Rule::new(
            "Save-down volume",
            &[("aws_finspace_kx_volume", &["name"])],
            false,
            &[],
        ),
        ("aws_finspace_kx_cluster", "tickerplant_log_configuration.tickerplant_log_volumes") => {
            Rule::new(
                "Tickerplant log volumes",
                &[("aws_finspace_kx_volume", &["name"])],
                true,
                &[],
            )
        }
        ("aws_finspace_kx_dataview", "segment_configurations.volume_name") => Rule::new(
            "Dataview segment volume",
            &[("aws_finspace_kx_volume", &["name"])],
            false,
            &["segment_configurations"],
        ),
        ("aws_finspace_kx_cluster", "code.s3_bucket") => Rule::new(
            "Cluster code bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_finspace_kx_environment"
            | "aws_finspace_kx_cluster"
            | "aws_finspace_kx_database"
            | "aws_finspace_kx_dataview"
            | "aws_finspace_kx_scaling_group"
            | "aws_finspace_kx_user"
            | "aws_finspace_kx_volume"
    )
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match r.attribute.as_str() {
        "vpc_configuration.subnet_ids"
        | "vpc_configuration.security_group_ids"
        | "vpc_configuration.vpc_id" => {
            return configured_links::network_context(
                e,
                r.target,
                &[
                    ("vpc_configuration.subnet_ids", "aws_subnet"),
                    ("vpc_configuration.security_group_ids", "aws_security_group"),
                ],
            )
            .zip(e.configured_endpoint(r.target, "vpc_configuration.0.vpc_id", "aws_vpc", &["id"]))
            .is_some_and(|(derived, declared)| derived == declared);
        }
        "database.database_name"
        | "database.dataview_name"
        | "database_name"
        | "scaling_group_configuration.scaling_group_name"
        | "savedown_storage_configuration.volume_name"
        | "tickerplant_log_configuration.tickerplant_log_volumes"
        | "segment_configurations.volume_name" => (),
        _ => return true,
    }
    if !e.same_endpoint(
        r.target,
        "environment_id",
        source,
        "environment_id",
        "aws_finspace_kx_environment",
        "id",
    ) {
        return false;
    }
    match r.attribute.as_str() {
        "database.dataview_name" => path.strip_suffix(".dataview_name").is_some_and(|parent| {
            e.same_endpoint(
                r.target,
                &format!("{parent}.database_name"),
                source,
                "database_name",
                "aws_finspace_kx_database",
                "name",
            )
        }),
        "segment_configurations.volume_name" => e
            .text(r.target, "availability_zone_id")
            .is_some_and(|zone| {
                e.value(source, "availability_zones")
                    .and_then(Value::as_array)
                    .is_some_and(|zones| {
                        zones.iter().all(Value::is_string)
                            && zones
                                .iter()
                                .any(|candidate| candidate.as_str() == Some(zone))
                    })
            }),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/finspace_kx_configuration.rs"]
mod tests;
