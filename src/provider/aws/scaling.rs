use super::service_evidence::{eligible_with_global_role, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/scaling.rs"]
mod tests;

type Rule = (&'static str, &'static [(&'static str, &'static str)], bool);

fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    Some(match (resource, attribute) {
        (
            "aws_autoscaling_group_tag"
            | "aws_autoscaling_lifecycle_hook"
            | "aws_autoscaling_schedule",
            "autoscaling_group_name",
        ) => (
            "Auto Scaling group",
            &[("aws_autoscaling_group", "name")],
            false,
        ),
        ("aws_autoscaling_lifecycle_hook", "notification_target_arn") => (
            "Lifecycle notification target",
            &[("aws_sns_topic", "arn"), ("aws_sqs_queue", "arn")],
            false,
        ),
        ("aws_autoscaling_lifecycle_hook", "role_arn") => (
            "Lifecycle notification role",
            &[("aws_iam_role", "arn")],
            false,
        ),
        ("aws_autoscaling_notification", "group_names") => (
            "Notification groups",
            &[("aws_autoscaling_group", "name")],
            true,
        ),
        ("aws_autoscaling_notification", "topic_arn") => {
            ("Notification topic", &[("aws_sns_topic", "arn")], false)
        }
        _ => return None,
    })
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.0)
}

fn basic(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    let Some((_, targets, list)) = rule(&nodes[r.target].resource_type, &r.attribute) else {
        return false;
    };
    if !eligible_with_global_role(nodes, r, r.attribute == "role_arn")
        || r.source_attributes.len() != 1
    {
        return false;
    }
    let field = r.source_attributes.iter().next().unwrap();
    let expected: Option<std::collections::BTreeSet<_>> = r
        .sources
        .iter()
        .map(|&i| {
            targets
                .contains(&(nodes[i].resource_type.as_str(), field.as_str()))
                .then(|| known(evidence, i, field))
                .flatten()
        })
        .collect();
    let Some(expected) = expected else {
        return false;
    };
    let (value, unknown) = evidence[r.target];
    expected.len() == r.sources.len()
        && super::streaming::values(value, unknown, &r.attribute, list)
            .is_some_and(|actual| !actual.is_empty() && actual == expected)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        if rule(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = basic(nodes, r, evidence);
        }
    }
}
