use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    node.references.iter().any(|reference| {
        reference.attribute == path
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
            && super::label(&node.resource_type, path) == Some(reference.label)
    })
}

#[test]
fn configured_links_require_current_typed_evidence() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/paymentcryptography-configuration-plan.json"
    ))
    .unwrap();
    let address = "aws_paymentcryptography_key_alias.sample1";
    assert!(verified(&fixture, address, "key_arn"));
    let mut masked = fixture.clone();
    masked["resource_changes"][1]["change"]["after_unknown"] = json!(true);
    assert!(!verified(&masked, address, "key_arn"));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["key_arn"] = Value::Null;
    assert!(!verified(
        &changed,
        "aws_paymentcryptography_key_alias.sample1",
        "key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_paymentcryptography_key_alias.sample1",
        "key_arn"
    ));
}

#[test]
fn member_owner_preserves_known_provider_account() {
    let parsed = plan::evidence::tests::parse_fixture(include_str!(
        "../../fixtures/paymentcryptography-configuration-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_paymentcryptography_key_alias.sample1")
        .unwrap();
    let owner = node.member_owner.as_ref().unwrap();
    assert_eq!(owner.target.address, "aws_paymentcryptography_key.sample0");
    assert_eq!(owner.section, "Aliases");
    assert_eq!(
        node.entity
            .scope
            .as_ref()
            .and_then(|scope| scope.account.as_ref())
            .map(|account| account.as_str()),
        Some("123456789012")
    );
}
