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
