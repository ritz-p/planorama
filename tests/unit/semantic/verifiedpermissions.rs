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
        "../../fixtures/verifiedpermissions-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_verifiedpermissions_policy.sample1", "policy_store_id"),
        (
            "aws_verifiedpermissions_policy.sample1",
            "definition.template_linked.policy_template_id",
        ),
        (
            "aws_verifiedpermissions_identity_source.sample5",
            "policy_store_id",
        ),
        (
            "aws_verifiedpermissions_identity_source.sample5",
            "configuration.cognito_user_pool_configuration.user_pool_arn",
        ),
        (
            "aws_verifiedpermissions_identity_source.sample5",
            "configuration.cognito_user_pool_configuration.client_ids",
        ),
    ] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][2]["change"]["after"]["policy_store_id"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_verifiedpermissions_policy.sample1",
        "definition.template_linked.policy_template_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][4]["change"]["after"]["user_pool_id"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_verifiedpermissions_identity_source.sample5",
        "configuration.cognito_user_pool_configuration.client_ids"
    ));
}
