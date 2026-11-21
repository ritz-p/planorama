use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_bcmdataexports_export",
            "export.destination_configurations.s3_destination.s3_bucket",
        ) => Rule::new(
            "Data export bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "export",
                "export.destination_configurations",
                "export.destination_configurations.s3_destination",
            ],
        )
        .in_control_plane_account(),
        ("aws_cur_report_definition", "s3_bucket") => Rule::new(
            "Cost and Usage Report bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_billing_view", "source_views") => Rule::new(
            "Source billing views",
            &[("aws_billing_view", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget", "billing_view_arn") => Rule::new(
            "Billing view",
            &[("aws_billing_view", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget", "notification.subscriber_sns_topic_arns") => Rule::new(
            "Budget notification topics",
            &[("aws_sns_topic", &["arn"])],
            true,
            &[],
        )
        .unordered_lists(&["notification"])
        .with_explicit_destination(),
        ("aws_budgets_budget_action", "budget_name") => Rule::new(
            "AWS budget",
            &[("aws_budgets_budget", &["name"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget_action", "execution_role_arn") => Rule::new(
            "Budget execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget_action", "definition.iam_action_definition.policy_arn") => Rule::new(
            "Budget IAM policy",
            &[("aws_iam_policy", &["arn"])],
            false,
            &["definition", "definition.iam_action_definition"],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget_action", "definition.scp_action_definition.policy_id") => Rule::new(
            "Budget service control policy",
            &[("aws_organizations_policy", &["id"])],
            false,
            &["definition", "definition.scp_action_definition"],
        )
        .in_control_plane_account(),
        ("aws_budgets_budget_action", "definition.ssm_action_definition.instance_ids") => {
            Rule::new(
                "Budget SSM action instances",
                &[
                    ("aws_instance", &["id"]),
                    ("aws_db_instance", &["identifier"]),
                ],
                true,
                &["definition", "definition.ssm_action_definition"],
            )
            .with_explicit_destination()
        }
        ("aws_ce_anomaly_subscription", "monitor_arn_list") => Rule::new(
            "Cost anomaly monitors",
            &[("aws_ce_anomaly_monitor", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies)
}

fn qualifies(
    evidence: &configured_links::Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
    path: &str,
) -> bool {
    let source = &evidence.nodes[reference.target];
    let target = &evidence.nodes[endpoint];
    match (source.resource_type.as_str(), reference.attribute.as_str()) {
        ("aws_bcmdataexports_export" | "aws_cur_report_definition", attribute)
            if attribute.ends_with("s3_bucket") =>
        {
            let (block, _) = path.rsplit_once('.').unwrap_or(("", path));
            let region_path = if block.is_empty() {
                "s3_region".to_owned()
            } else {
                format!("{block}.s3_region")
            };
            let scope = super::scope::scope(target);
            evidence
                .text(reference.target, &region_path)
                .zip(scope)
                .is_some_and(|(region, scope)| {
                    scope.region.as_ref().map(|region| region.0.as_str()) == Some(region)
                })
        }
        ("aws_budgets_budget_action", "budget_name") => {
            let configured_accounts = evidence
                .text(reference.target, "account_id")
                .zip(evidence.text(endpoint, "account_id"));
            configured_accounts.map_or_else(
                || {
                    !evidence.has_argument(reference.target, "account_id")
                        && !evidence.has_argument(endpoint, "account_id")
                        && configured_links::same_account(
                            source.provider_configuration.as_ref(),
                            target.provider_configuration.as_ref(),
                        )
                },
                |(left, right)| left == right,
            )
        }
        ("aws_budgets_budget_action", "definition.scp_action_definition.policy_id") => {
            evidence.text(endpoint, "type") == Some("SERVICE_CONTROL_POLICY")
        }
        ("aws_budgets_budget_action", "definition.ssm_action_definition.instance_ids") => {
            let Some((block, _)) = path.rsplit_once('.') else {
                return false;
            };
            let action = evidence.text(reference.target, &format!("{block}.action_sub_type"));
            if !matches!(
                (action, target.resource_type.as_str()),
                (Some("STOP_EC2_INSTANCES"), "aws_instance")
                    | (Some("STOP_RDS_INSTANCES"), "aws_db_instance")
            ) {
                return false;
            }
            let region = evidence.text(reference.target, &format!("{block}.region"));
            let scope = super::scope::scope(target);
            region.zip(scope).is_some_and(|(region, scope)| {
                scope.region.as_ref().map(|region| region.0.as_str()) == Some(region)
                    && destination_account_matches(evidence, reference, endpoint)
            })
        }
        ("aws_budgets_budget", "notification.subscriber_sns_topic_arns")
        | ("aws_ce_anomaly_subscription", "monitor_arn_list") => {
            destination_account_matches(evidence, reference, endpoint)
        }
        _ => true,
    }
}

fn destination_account_matches(
    evidence: &configured_links::Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
) -> bool {
    let source = evidence.nodes[reference.target]
        .provider_configuration
        .as_ref();
    let target = evidence.nodes[endpoint].provider_configuration.as_ref();
    if let Some(account) = evidence.text(reference.target, "account_id") {
        return target
            .and_then(|config| config.account.as_ref())
            .is_some_and(|scope| scope.as_str() == account);
    }
    !evidence.has_argument(reference.target, "account_id")
        && configured_links::same_account(source, target)
}
