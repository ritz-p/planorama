use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const SLACK: &str = "aws_chatbot_slack_channel_configuration";
const TEAMS: &str = "aws_chatbot_teams_channel_configuration";

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (SLACK | TEAMS, "iam_role_arn") => Rule::new(
            "Chatbot channel role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (SLACK | TEAMS, "guardrail_policy_arns") => Rule::new(
            "Chatbot guardrail policies",
            &[("aws_iam_policy", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        (SLACK | TEAMS, "sns_topic_arns") => Rule::new(
            "Chatbot notification topics",
            &[("aws_sns_topic", &["arn"])],
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

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}

fn arn<'a>(
    node: &TerraformEntity,
    value: &'a str,
    service: &str,
    prefix: &str,
) -> Option<(&'a str, &'a str)> {
    let mut fields = value.splitn(6, ':');
    if fields.next()? != "arn" || !fields.next()?.starts_with("aws") || fields.next()? != service {
        return None;
    }
    let region = fields.next()?;
    let account = fields.next()?;
    let identity = fields.next()?.strip_prefix(prefix)?;
    let binding = node.provider_configuration.as_ref()?;
    (!identity.is_empty()
        && !binding.account_conflict
        && crate::model::AwsAccountScope::new(account).is_some()
        && binding
            .account
            .as_ref()
            .is_none_or(|a| a.as_str() == account))
    .then_some((region, identity))
}

fn qualified_arn(node: &TerraformEntity, value: &str, service: &str, prefix: &str) -> bool {
    arn(node, value, service, prefix).is_some_and(|(region, _)| {
        let scope = super::scope::scope(node);
        if service == "iam" {
            region.is_empty()
        } else {
            scope.and_then(|s| s.region).is_some_and(|s| s.0 == region)
        }
    })
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    let (service, prefix) = match r.attribute.as_str() {
        "iam_role_arn" => ("iam", "role/"),
        "guardrail_policy_arns" => ("iam", "policy/"),
        "sns_topic_arns" => ("sns", ""),
        _ => return false,
    };
    e.text(source, "arn")
        .is_some_and(|value| qualified_arn(&e.nodes[source], value, service, prefix))
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/communications.rs"]
mod tests;
