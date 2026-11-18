use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_applicationinsights_application", "resource_group_name") => Rule::new(
            "Application resource group",
            &[("aws_resourcegroups_group", &["name"])],
            false,
            &[],
        ),
        ("aws_applicationinsights_application", "ops_item_sns_topic_arn") => {
            Rule::new("OpsItem topic", &[("aws_sns_topic", &["arn"])], false, &[])
        }
        ("aws_evidently_feature" | "aws_evidently_launch", "project") => Rule::new(
            "Evidently project",
            &[("aws_evidently_project", &["name", "arn"])],
            false,
            &[],
        ),
        ("aws_evidently_launch", "groups.feature") => Rule::new(
            "Launch feature",
            &[("aws_evidently_feature", &["name"])],
            false,
            &["groups"],
        ),
        ("aws_evidently_launch", "scheduled_splits_config.steps.segment_overrides.segment") => {
            Rule::new(
                "Launch segment",
                &[("aws_evidently_segment", &["name", "arn"])],
                false,
                &[
                    "scheduled_splits_config.steps",
                    "scheduled_splits_config.steps.segment_overrides",
                ],
            )
        }
        ("aws_evidently_project", "data_delivery.cloudwatch_logs.log_group") => Rule::new(
            "Experiment log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[],
        ),
        ("aws_evidently_project", "data_delivery.s3_destination.bucket") => Rule::new(
            "Experiment results bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_internetmonitor_monitor", "resources") => Rule::new(
            "Monitored resources",
            &[
                ("aws_vpc", &["arn"]),
                ("aws_lb", &["arn"]),
                ("aws_alb", &["arn"]),
                ("aws_cloudfront_distribution", &["arn"]),
                ("aws_workspaces_directory", &["arn"]),
            ],
            true,
            &[],
        ),
        (
            "aws_internetmonitor_monitor",
            "internet_measurements_log_delivery.s3_config.bucket_name",
        ) => Rule::new(
            "Measurements bucket",
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
        "aws_applicationinsights_application"
            | "aws_evidently_feature"
            | "aws_evidently_launch"
            | "aws_evidently_project"
            | "aws_internetmonitor_monitor"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    match r.attribute.as_str() {
        "resource_group_name" => e
            .value(r.target, "grouping_type")
            .is_some_and(Value::is_null),
        "groups.feature" => ["name", "arn"].into_iter().any(|field| {
            e.same_endpoint(
                r.target,
                "project",
                source,
                "project",
                "aws_evidently_project",
                field,
            )
        }),
        "resources" => {
            let family = |kind: &str| match kind {
                "aws_vpc" | "aws_cloudfront_distribution" => 1,
                "aws_lb" | "aws_alb" => 2,
                "aws_workspaces_directory" => 3,
                _ => 0,
            };
            let group = family(&e.nodes[source].resource_type);
            group != 0
                && r.sources
                    .iter()
                    .all(|&index| family(&e.nodes[index].resource_type) == group)
                && (group != 2 || e.text(source, "load_balancer_type") == Some("network"))
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/observability_monitors.rs"]
mod tests;
