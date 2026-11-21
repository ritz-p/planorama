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
fn lex_bot_owns_scoped_draft_locale() {
    let mut fixture = plan(&[
        ("aws_lexv2models_bot.main", json!({"id":"bot"}), json!({})),
        (
            "aws_lexv2models_bot_locale.english",
            json!({"bot_id":"bot","bot_version":"DRAFT","locale_id":"en_US"}),
            json!({"bot_id":{"references":["aws_lexv2models_bot.main.id"]}}),
        ),
    ]);
    let owner = |v: &Value| {
        graph(v)
            .nodes
            .iter()
            .find(|n| n.address == "aws_lexv2models_bot_locale.english")
            .unwrap()
            .member_owner
            .is_some()
    };
    assert!(owner(&fixture));
    fixture["resource_changes"][1]["change"]["after_sensitive"] = json!({"bot_id":true});
    assert!(!owner(&fixture));
}

#[test]
fn lex_source_versions_use_terraform_map_expression_references() {
    let mut fixture = plan(&[
        ("aws_lexv2models_bot.main", json!({"id":"bot"}), json!({})),
        (
            "aws_lexv2models_bot_version.previous",
            json!({"bot_id":"bot","bot_version":"1","locale_specification":{"en_US":{"source_bot_version":"DRAFT"}}}),
            json!({"bot_id":{"references":["aws_lexv2models_bot.main.id"]}}),
        ),
        (
            "aws_lexv2models_bot_version.next",
            json!({"bot_id":"bot","locale_specification":{"en_US":{"source_bot_version":"1"}}}),
            json!({"bot_id":{"references":["aws_lexv2models_bot.main.id"]},"locale_specification":{"references":["aws_lexv2models_bot_version.previous.bot_version"]}}),
        ),
    ]);
    let check = |v: &Value| {
        crate::configured_service_links::verified(
            &graph(v),
            "aws_lexv2models_bot_version.next",
            "locale_specification",
            Some("Lex source locale version"),
        )
    };
    assert!(check(&fixture));
    fixture["resource_changes"][2]["change"]["after_unknown"] =
        json!({"locale_specification":{"en_US":{"source_bot_version":true}}});
    assert!(!check(&fixture));
}

#[test]
fn lex_intent_owns_only_matching_locale() {
    let mut fixture = plan(&[
        ("aws_lexv2models_bot.main", json!({"id":"bot"}), json!({})),
        (
            "aws_lexv2models_bot_locale.english",
            json!({"bot_id":"bot","bot_version":"DRAFT","locale_id":"en_US"}),
            json!({"bot_id":{"references":["aws_lexv2models_bot.main.id"]}}),
        ),
        (
            "aws_lexv2models_intent.main",
            json!({"bot_id":"bot","bot_version":"DRAFT","locale_id":"en_US"}),
            json!({
                "bot_id":{"references":["aws_lexv2models_bot.main.id"]},"locale_id":{"references":["aws_lexv2models_bot_locale.english.locale_id"]}
            }),
        ),
    ]);
    let owner = |v: &Value| {
        graph(v)
            .nodes
            .iter()
            .find(|n| n.address == "aws_lexv2models_intent.main")
            .unwrap()
            .member_owner
            .is_some()
    };
    assert!(owner(&fixture));
    fixture["resource_changes"][2]["change"]["after"]["bot_version"] = json!("1");
    assert!(!owner(&fixture));
}

#[test]
fn notification_channel_separates_customer_and_managed_configurations() {
    let config = "arn:aws:notifications::123456789012:notification-configuration/test";
    let channel = "arn:aws:notificationscontacts:us-west-2:123456789012:emailcontact:test";
    let mut fixture = plan(&[
        (
            "aws_notifications_notification_configuration.main",
            json!({"arn":config}),
            json!({}),
        ),
        (
            "aws_notificationscontacts_email_contact.main",
            json!({"arn":channel}),
            json!({}),
        ),
        (
            "aws_notifications_channel_association.main",
            json!({"arn":channel,"notification_configuration_arn":config}),
            json!({
                "arn":{"references":["aws_notificationscontacts_email_contact.main.arn"]},
                "notification_configuration_arn":{"references":["aws_notifications_notification_configuration.main.arn"]}
            }),
        ),
    ]);
    let check = |v: &Value, path: &str, label| {
        crate::configured_service_links::verified(
            &graph(v),
            "aws_notifications_channel_association.main",
            path,
            Some(label),
        )
    };
    assert!(check(&fixture, "arn", "Notification channel"));
    assert!(check(
        &fixture,
        "notification_configuration_arn",
        "Notification configuration"
    ));
    let managed =
        "arn:aws:notifications::123456789012:managed-notification-configuration/category/test";
    fixture["resource_changes"][0]["change"]["after"]["arn"] = json!(managed);
    fixture["resource_changes"][2]["change"]["after"]["notification_configuration_arn"] =
        json!(managed);
    assert!(!check(
        &fixture,
        "notification_configuration_arn",
        "Notification configuration"
    ));
}
