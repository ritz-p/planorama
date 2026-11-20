use super::label;
use crate::configured_service_links::{check, verified};
use serde_json::{Value, json};

const PROVIDER: &str = "registry.terraform.io/hashicorp/aws";

fn plan(resources: &[(&str, Value, Value)]) -> Value {
    let changes: Vec<_> = resources
        .iter()
        .map(|(address, values, _)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_name": PROVIDER, "change": {"actions": ["create"], "after": values}
            })
        })
        .collect();
    let config: Vec<_> = resources
        .iter()
        .map(|(address, _, expressions)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_config_key": "aws", "expressions": expressions
            })
        })
        .collect();
    json!({
        "format_version": "1.2", "terraform_version": "1.9.8", "resource_changes": changes,
        "configuration": {
            "provider_config": {"aws": {"name": "aws", "full_name": PROVIDER, "expressions": {
                "region": {"constant_value": "us-east-1"},
                "assume_role": [{"role_arn": {"constant_value": "arn:aws:iam::123456789012:role/Provider"}}]
            }}},
            "root_module": {"resources": config}
        }
    })
}

fn graph(plan: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&plan.to_string()).unwrap())
}

#[test]
fn chatbot_uses_configured_managed_endpoints() {
    let role = "arn:aws:iam::123456789012:role/Chat";
    let policy = "arn:aws:iam::123456789012:policy/Chat";
    let topic = "arn:aws:sns:us-east-1:123456789012:updates";
    for kind in [super::SLACK, super::TEAMS] {
        let address = format!("{kind}.channel");
        let fixture = plan(&[
            ("aws_iam_role.chat", json!({"arn":role}), json!({})),
            ("aws_iam_policy.chat", json!({"arn":policy}), json!({})),
            ("aws_sns_topic.updates", json!({"arn":topic}), json!({})),
            (
                &address,
                json!({"iam_role_arn":role,"guardrail_policy_arns":[policy],"sns_topic_arns":[topic]}),
                json!({
                    "iam_role_arn":{"references":["aws_iam_role.chat.arn"]},
                    "guardrail_policy_arns":{"references":["aws_iam_policy.chat.arn"]},
                    "sns_topic_arns":{"references":["aws_sns_topic.updates.arn"]}
                }),
            ),
        ]);
        for (path, endpoint) in [
            ("iam_role_arn", "aws_iam_role.chat"),
            ("guardrail_policy_arns", "aws_iam_policy.chat"),
            ("sns_topic_arns", "aws_sns_topic.updates"),
        ] {
            check(
                &fixture.to_string(),
                crate::plan::parse,
                label,
                &[(&address, path)],
                (endpoint, "arn"),
            );
        }
        assert!(label(kind, "slack_channel_id").is_none());
        assert!(label(kind, "tenant_id").is_none());
    }
}

#[test]
fn communications_codestar_matches_the_provider_chat_output() {
    let source = include_str!("../../fixtures/delivery-notifications-references-plan.json");
    let consumer = "aws_codestarnotifications_notification_rule.delivery";
    let check = |value: &Value| {
        verified(
            &graph(value),
            consumer,
            "target.address",
            Some("Notification target"),
        )
    };
    for (kind, selector) in [
        (super::SLACK, "AWSChatbotSlack"),
        (super::TEAMS, "AWSChatbotMicrosoftTeams"),
    ] {
        let value: Value = serde_json::from_str(
            &source
                .replace(super::SLACK, kind)
                .replace("AWSChatbotSlack", selector),
        )
        .unwrap();
        assert!(check(&value));
        let wrong: Value =
            serde_json::from_str(&value.to_string().replace(".chat_configuration_arn", ".arn"))
                .unwrap();
        assert!(!check(&wrong));
    }
}

#[test]
fn communications_voice_connector_placement_and_unordered_members() {
    let mut fixture = plan(&[
        (
            "aws_chime_voice_connector.west",
            json!({"id":"west","aws_region":"us-west-2","arn":"arn:aws:chime:us-west-2:123456789012:vc/west"}),
            json!({}),
        ),
        (
            "aws_chime_voice_connector.east",
            json!({"id":"east","arn":"arn:aws:chime:us-east-1:123456789012:vc/east"}),
            json!({}),
        ),
        (
            "aws_chime_voice_connector_group.both",
            json!({"connector":[{"voice_connector_id":"east","priority":2},{"voice_connector_id":"west","priority":1}]}),
            json!({
                "connector":[{"voice_connector_id":{"references":["aws_chime_voice_connector.west.id"]}},{"voice_connector_id":{"references":["aws_chime_voice_connector.east.id"]}}]
            }),
        ),
        (
            "aws_chime_voice_connector_termination_credentials.auth",
            json!({"voice_connector_id":"west","credentials":[{"username":"PRIVATE_CREDENTIAL","password":"PRIVATE_PASSWORD"}]}),
            json!({
                "voice_connector_id":{"references":["aws_chime_voice_connector.west.id"]}
            }),
        ),
    ]);
    let west = "aws_chime_voice_connector.west";
    let credentials = "aws_chime_voice_connector_termination_credentials.auth";
    let group = "aws_chime_voice_connector_group.both";
    let parsed = graph(&fixture);
    let node = |address| parsed.nodes.iter().find(|n| n.address == address).unwrap();
    assert_eq!(
        node(west)
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "us-west-2"
    );
    assert_eq!(
        node(credentials).member_owner.as_ref().unwrap().attribute,
        "voice_connector_id"
    );
    assert!(verified(
        &parsed,
        group,
        "connector.voice_connector_id",
        label(super::VOICE_GROUP, "connector.voice_connector_id")
    ));
    fixture["resource_changes"][0]["change"]["after"]["aws_region"] = json!("us-east-1");
    let parsed = graph(&fixture);
    assert!(!verified(
        &parsed,
        group,
        "connector.voice_connector_id",
        label(super::VOICE_GROUP, "connector.voice_connector_id")
    ));
    assert!(
        parsed
            .nodes
            .iter()
            .find(|n| n.address == credentials)
            .unwrap()
            .member_owner
            .is_none()
    );
}
