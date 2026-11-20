use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const SLACK: &str = "aws_chatbot_slack_channel_configuration";
const TEAMS: &str = "aws_chatbot_teams_channel_configuration";

const VOICE_CONNECTOR: &str = "aws_chime_voice_connector";
const VOICE_GROUP: &str = "aws_chime_voice_connector_group";
const VOICE_LOGGING: &str = "aws_chime_voice_connector_logging";
const VOICE_ORIGINATION: &str = "aws_chime_voice_connector_origination";
const VOICE_TERMINATION: &str = "aws_chime_voice_connector_termination";
const VOICE_CREDENTIALS: &str = "aws_chime_voice_connector_termination_credentials";

fn voice_configuration(kind: &str) -> bool {
    matches!(
        kind,
        VOICE_LOGGING | VOICE_ORIGINATION | VOICE_TERMINATION | VOICE_CREDENTIALS
    )
}

pub(crate) fn has_indexed(kind: &str) -> bool {
    kind == VOICE_GROUP
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    has_indexed(kind) && configured_links::indexed(kind, path, rule)
}

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
        (kind, "voice_connector_id") if voice_configuration(kind) => Rule::new(
            "Chime voice connector",
            &[(VOICE_CONNECTOR, &["id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (VOICE_GROUP, "connector.voice_connector_id") => Rule::scalar_set(
            "Chime voice connector members",
            &[(VOICE_CONNECTOR, &["id"])],
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
    if e.nodes[source].resource_type == VOICE_CONNECTOR {
        return e.nodes[source].region_override.is_some();
    }
    let (service, prefix) = match r.attribute.as_str() {
        "iam_role_arn" => ("iam", "role/"),
        "guardrail_policy_arns" => ("iam", "policy/"),
        "sns_topic_arns" => ("sns", ""),
        _ => return false,
    };
    e.text(source, "arn")
        .is_some_and(|value| qualified_arn(&e.nodes[source], value, service, prefix))
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == crate::model::EntityMode::Managed
        && node.action != crate::model::Action::Delete
        && node.deposed_key.is_none()
}

pub(crate) fn service_regions(nodes: &mut [TerraformEntity], values: &[(&Value, &Value)]) {
    for (index, node) in nodes.iter_mut().enumerate() {
        let prefix = match node.resource_type.as_str() {
            VOICE_CONNECTOR => "vc/",
            _ => continue,
        };
        if !current(node) {
            continue;
        }
        let read = |field| super::service_evidence::known(values, index, field);
        let selected = read("aws_region").and_then(super::scope_selections::region);
        let observed = read("arn")
            .and_then(|value| arn(node, value, "chime", prefix))
            .filter(|&(_, id)| read("id") == Some(id))
            .and_then(|(region, _)| super::scope_selections::region(region));
        let selected = match (selected, observed, read("arn")) {
            (Some(a), Some(b), _) if a != b => None,
            (_, None, Some(_)) => None,
            (a, b, _) => a.or(b),
        };
        node.has_region_override = true;
        node.region_override = selected.map(|r| crate::model::RegionScope(r.to_owned()));
    }
}

pub(crate) fn inherit_voice_region(nodes: &mut [TerraformEntity], refs: &[AttributeReference]) {
    for r in refs {
        if voice_configuration(&nodes[r.target].resource_type)
            && r.attribute == "voice_connector_id"
            && r.service_reference_verified
        {
            if let [source] = r.sources.as_slice() {
                nodes[r.target].has_region_override = true;
                nodes[r.target].region_override = nodes[*source].region_override.clone();
            }
        }
    }
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    voice_configuration(kind).then_some((
        "voice_connector_id",
        "Chime voice connector",
        "Connector settings",
    ))
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/communications.rs"]
mod tests;
