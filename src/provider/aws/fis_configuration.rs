use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_fis_experiment_template", "role_arn") => {
            Rule::new("Experiment role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_fis_experiment_template", "target.resource_arns") => Rule::new(
            "Explicit experiment targets",
            &[
                ("aws_instance", &["arn"]),
                ("aws_subnet", &["arn"]),
                ("aws_ebs_volume", &["arn"]),
                ("aws_ecs_cluster", &["arn"]),
                ("aws_autoscaling_group", &["arn"]),
                ("aws_ec2_transit_gateway", &["arn"]),
                ("aws_vpc_endpoint", &["arn"]),
                ("aws_eks_node_group", &["arn"]),
                ("aws_kinesis_stream", &["arn"]),
                ("aws_eks_cluster", &["arn"]),
                ("aws_rds_cluster", &["arn"]),
                ("aws_db_instance", &["arn"]),
                ("aws_lambda_function", &["arn"]),
                ("aws_elasticache_replication_group", &["arn"]),
                ("aws_iam_role", &["arn"]),
                ("aws_s3_bucket", &["arn"]),
            ],
            true,
            &["target"],
        ),
        ("aws_fis_experiment_template", "stop_condition.value") => Rule::new(
            "Experiment stop alarm",
            &[
                ("aws_cloudwatch_metric_alarm", &["arn"]),
                ("aws_cloudwatch_composite_alarm", &["arn"]),
            ],
            false,
            &["stop_condition"],
        ),
        (
            "aws_fis_experiment_template",
            "log_configuration.cloudwatch_logs_configuration.log_group_arn",
        ) => Rule::new(
            "Experiment log group",
            &[("aws_cloudwatch_log_group", &["arn"])],
            false,
            &[],
        ),
        ("aws_fis_experiment_template", "log_configuration.s3_configuration.bucket_name") => {
            Rule::new(
                "Experiment log bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
        }
        (
            "aws_fis_experiment_template",
            "experiment_report_configuration.data_sources.cloudwatch_dashboard.dashboard_arn",
        ) => Rule::new(
            "Experiment report dashboard",
            &[("aws_cloudwatch_dashboard", &["dashboard_arn"])],
            false,
            &["experiment_report_configuration.data_sources.cloudwatch_dashboard"],
        ),
        (
            "aws_fis_experiment_template",
            "experiment_report_configuration.outputs.s3_configuration.bucket_name",
        ) => Rule::new(
            "Experiment report bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_fis_target_account_configuration", "experiment_template_id") => Rule::new(
            "Experiment template",
            &[("aws_fis_experiment_template", &["id"])],
            false,
            &[],
        ),
        ("aws_fis_target_account_configuration", "account_id") => Rule::new(
            "Experiment target account",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        ),
        ("aws_fis_target_account_configuration", "role_arn") => Rule::new(
            "Target account role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_fis_experiment_template" | "aws_fis_target_account_configuration"
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
        "target.resource_arns" => path
            .rsplit_once('.')
            .filter(|(p, _)| {
                ["filter", "resource_tag", "parameters"]
                    .iter()
                    .all(|field| e.absent(r.target, &format!("{p}.{field}")))
            })
            .and_then(|(p, _)| e.text(r.target, &format!("{p}.resource_type")))
            .is_some_and(|kind| {
                matches!(
                    (kind, e.nodes[source].resource_type.as_str()),
                    ("aws:ec2:instance", "aws_instance")
                        | ("aws:ec2:subnet", "aws_subnet")
                        | ("aws:ec2:ebs-volume", "aws_ebs_volume")
                        | ("aws:ecs:cluster", "aws_ecs_cluster")
                        | ("aws:ec2:autoscaling-group", "aws_autoscaling_group")
                        | ("aws:ec2:transit-gateway", "aws_ec2_transit_gateway")
                        | ("aws:ec2:vpc-endpoint", "aws_vpc_endpoint")
                        | ("aws:eks:nodegroup", "aws_eks_node_group")
                        | ("aws:kinesis:stream", "aws_kinesis_stream")
                        | ("aws:eks:cluster", "aws_eks_cluster")
                        | ("aws:rds:cluster", "aws_rds_cluster")
                        | ("aws:rds:db", "aws_db_instance")
                        | ("aws:lambda:function", "aws_lambda_function")
                        | (
                            "aws:elasticache:replicationgroup",
                            "aws_elasticache_replication_group"
                        )
                        | ("aws:iam:role", "aws_iam_role")
                        | ("aws:s3:bucket", "aws_s3_bucket")
                )
            }),
        "stop_condition.value" => path.rsplit_once('.').is_some_and(|(p, _)| {
            e.text(r.target, &format!("{p}.source")) == Some("aws:cloudwatch:alarm")
        }),
        "role_arn" if e.nodes[r.target].resource_type == "aws_fis_target_account_configuration" => {
            e.text(r.target, "account_id")
                .zip(e.text(source, "arn"))
                .is_some_and(|(a, arn)| {
                    crate::model::AwsAccountScope::new(a).is_some()
                        && arn.split(':').nth(4) == Some(a)
                        && e.nodes[source]
                            .provider_configuration
                            .as_ref()
                            .and_then(|p| p.account.as_ref())
                            .is_some_and(|actual| actual.as_str() == a)
                })
        }
        "account_id" => e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some(),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/fis_configuration.rs"]
mod tests;
