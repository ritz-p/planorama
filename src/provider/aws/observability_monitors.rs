use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MonitorFamily {
    Network,
    LoadBalancer,
    WorkSpaces,
}

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
                ("aws_workspaces_directory", &["id", "directory_id"]),
            ],
            true,
            &[],
        )
        .decoded(monitor_identifiers),
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
        "groups.feature" => {
            let project = |node| {
                e.configured_endpoint(node, "project", "aws_evidently_project", &["name", "arn"])
            };
            project(r.target)
                .zip(project(source))
                .is_some_and(|(launch_project, feature_project)| launch_project == feature_project)
        }
        "resources" => {
            let Some(family) = monitor_family(&e.nodes[source].resource_type) else {
                return false;
            };
            r.sources
                .iter()
                .all(|&index| monitor_family(&e.nodes[index].resource_type) == Some(family))
                && match family {
                    MonitorFamily::Network => true,
                    MonitorFamily::LoadBalancer => {
                        e.text(source, "load_balancer_type") == Some("network")
                    }
                    MonitorFamily::WorkSpaces => workspace_scope(e, r.target, source),
                }
        }
        _ => true,
    }
}
fn monitor_family(kind: &str) -> Option<MonitorFamily> {
    match kind {
        "aws_vpc" | "aws_cloudfront_distribution" => Some(MonitorFamily::Network),
        "aws_lb" | "aws_alb" => Some(MonitorFamily::LoadBalancer),
        "aws_workspaces_directory" => Some(MonitorFamily::WorkSpaces),
        _ => None,
    }
}

fn workspace_directory(arn: &str) -> Option<(&str, &str, &str)> {
    let mut parts = arn.splitn(6, ':');
    let [
        Some("arn"),
        Some("aws"),
        Some("workspaces"),
        Some(region),
        Some(account),
        Some(resource),
    ] = std::array::from_fn(|_| parts.next())
    else {
        return None;
    };
    let id = resource.strip_prefix("directory/")?;
    (id.strip_prefix("d-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
    }) && crate::model::AwsAccountScope::new(account).is_some())
    .then_some((region, account, id))
}

fn monitor_identifiers<'a>(_: &str, actual: &'a Value) -> Option<BTreeSet<&'a str>> {
    let items = actual.as_array()?;
    let identifiers = items
        .iter()
        .map(|item| {
            let arn = item.as_str()?;
            if let Some((_, _, directory)) = workspace_directory(arn) {
                Some(directory)
            } else {
                arn.starts_with("arn:").then_some(arn)
            }
        })
        .collect::<Option<BTreeSet<_>>>()?;
    (identifiers.len() == items.len()).then_some(identifiers)
}

fn workspace_scope(e: &Evidence<'_>, monitor: usize, directory: usize) -> bool {
    let Some(region) = super::scope::scope(&e.nodes[directory]).and_then(|scope| scope.region)
    else {
        return false;
    };
    let configured = e.nodes[directory]
        .provider_configuration
        .as_ref()
        .and_then(|provider| provider.account.as_ref());
    let observed = e.text(monitor, "arn").and_then(|arn| {
        let mut parts = arn.splitn(6, ':');
        let [
            Some("arn"),
            Some("aws"),
            Some("internetmonitor"),
            Some(arn_region),
            Some(account),
            Some(resource),
        ] = std::array::from_fn(|_| parts.next())
        else {
            return None;
        };
        (arn_region == region.0
            && resource
                .strip_prefix("monitor/")
                .is_some_and(|name| e.text(monitor, "monitor_name") == Some(name)))
        .then_some(account)
    });
    let account = configured
        .map(crate::model::AwsAccountScope::as_str)
        .or(observed);
    account.is_some_and(|account| {
        observed.is_none_or(|observed| observed == account)
            && e.value(monitor, "resources")
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().all(|item| {
                        item.as_str().and_then(workspace_directory).is_some_and(
                            |(arn_region, arn_account, _)| {
                                arn_region == region.0 && arn_account == account
                            },
                        )
                    })
                })
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/observability_monitors.rs"]
mod tests;
