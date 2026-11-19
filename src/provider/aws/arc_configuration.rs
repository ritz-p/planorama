use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind,path) {
        ("aws_arcregionswitch_plan", "execution_role")=>Rule::new("Region Switch execution role", &[("aws_iam_role", &["arn"])], false, &[]).with_explicit_destination(),
        ("aws_arcregionswitch_plan", "associated_alarms.resource_identifier")=>Rule::new("Plan alarm", &[("aws_cloudwatch_metric_alarm", &["arn"]),("aws_cloudwatch_composite_alarm", &["arn"])], false, &["associated_alarms"]).with_explicit_destination(),
        ("aws_arcregionswitch_plan", "associated_alarms.cross_account_role")=>Rule::new("Alarm access role", &[("aws_iam_role", &["arn"])], false, &["associated_alarms"]).with_explicit_destination(),
        ("aws_arcregionswitch_plan", "workflow.step.arc_routing_control_config.region_and_routing_controls.routing_control.routing_control_arn")=>Rule::new("Plan routing control", &[("aws_route53recoverycontrolconfig_routing_control", &["arn"])], false, &["workflow","workflow.step","workflow.step.arc_routing_control_config","workflow.step.arc_routing_control_config.region_and_routing_controls","workflow.step.arc_routing_control_config.region_and_routing_controls.routing_control"]).with_explicit_destination(),
        ("aws_arcregionswitch_plan", "workflow.step.custom_action_lambda_config.lambda.arn")=>Rule::new("Plan Lambda function", &[("aws_lambda_function", &["arn"])], false, &["workflow","workflow.step","workflow.step.custom_action_lambda_config","workflow.step.custom_action_lambda_config.lambda"]).with_explicit_destination(),
        ("aws_arczonalshift_zonal_autoshift_configuration", "resource_arn")=>Rule::new("Zonal shift resource", &[("aws_lb", &["arn"]),("aws_autoscaling_group", &["arn"]),("aws_eks_cluster", &["arn"])], false, &[]),
        ("aws_arczonalshift_zonal_autoshift_configuration", "blocking_alarms.alarm_identifier")=>Rule::new("Blocking alarm", &[("aws_cloudwatch_metric_alarm", &["arn"]),("aws_cloudwatch_composite_alarm", &["arn"])], false, &["blocking_alarms"]),
        ("aws_arczonalshift_zonal_autoshift_configuration", "outcome_alarms.alarm_identifier")=>Rule::new("Outcome alarm", &[("aws_cloudwatch_metric_alarm", &["arn"]),("aws_cloudwatch_composite_alarm", &["arn"])], false, &["outcome_alarms"]),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_arcregionswitch_plan" | "aws_arczonalshift_zonal_autoshift_configuration"
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
    if e.nodes[r.target].resource_type == "aws_arczonalshift_zonal_autoshift_configuration" {
        return match r.attribute.as_str() {
            "resource_arn" if e.nodes[source].resource_type == "aws_lb" => matches!(
                e.text(source, "load_balancer_type"),
                Some("application" | "network")
            ),
            "resource_arn" => true,
            _ => path
                .rsplit_once('.')
                .is_some_and(|(p, _)| e.text(r.target, &format!("{p}.type")) == Some("CLOUDWATCH")),
        };
    }
    let Some(arn) = e.text(source, "arn") else {
        return false;
    };
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, _, region, account, _] = parts.as_slice() else {
        return false;
    };
    let Some(binding) = e.nodes[source].provider_configuration.as_ref() else {
        return false;
    };
    let Some(consumer) = e.nodes[r.target].provider_configuration.as_ref() else {
        return false;
    };
    if binding
        .account
        .as_ref()
        .is_some_and(|a| a.as_str() != *account)
        || crate::model::AwsAccountScope::new(account).is_none()
    {
        return false;
    }
    let same = consumer
        .account
        .as_ref()
        .is_some_and(|a| a.as_str() == *account)
        || consumer == binding;
    let regions = e.value(r.target, "regions").and_then(Value::as_array);
    let in_plan = |s: &str| regions.is_some_and(|rs| rs.iter().any(|r| r.as_str() == Some(s)));
    if !region.is_empty()
        && (!in_plan(region) || binding.region.as_ref().is_none_or(|v| v.0 != *region))
    {
        return false;
    }
    let prefix = path.rsplit_once('.').map(|(p, _)| p);
    if r.attribute.ends_with("routing_control_arn") {
        let Some((row, _)) = path.rsplit_once(".routing_control.") else {
            return false;
        };
        if !e
            .text(r.target, &format!("{row}.region"))
            .is_some_and(in_plan)
        {
            return false;
        }
    }
    let role_path = if r.attribute.ends_with("routing_control_arn") {
        path.split_once(".region_and_routing_controls.")
            .map(|(p, _)| format!("{p}.cross_account_role"))
    } else {
        prefix.map(|p| format!("{p}.cross_account_role"))
    };
    let Some(role_path) = role_path else {
        return same;
    };
    let role = e.value(r.target, &role_path);
    if role.is_some_and(Value::is_null) {
        return same;
    }
    role.and_then(Value::as_str)
        .is_some_and(|v| v.starts_with(&format!("arn:{partition}:iam::{account}:role/")))
        && (same || (consumer.account.is_some() && binding.account.is_some()))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/arc_configuration.rs"]
mod tests;
