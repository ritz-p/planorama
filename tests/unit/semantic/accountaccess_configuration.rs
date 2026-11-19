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
        "../../fixtures/accountaccess-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_accountaccess_application.sample0",
            "identity_source.identity_center.instance_arn",
        ),
        ("aws_accountaccess_entitlement.sample1", "application_arn"),
        (
            "aws_accountaccess_entitlement.sample1",
            "entitlement.principal_role.role_arn",
        ),
        (
            "aws_accountaccess_entitlement.sample1",
            "entitlement.principal_role.principal.identity_center.user_id",
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
    changed["resource_changes"][4]["change"]["after"]["identity_store_id"] = json!("d-0987654321");
    assert!(!verified(
        &changed,
        "aws_accountaccess_entitlement.sample1",
        "entitlement.principal_role.principal.identity_center.user_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["arns"]
        .as_array_mut()
        .unwrap()
        .push(json!("arn:aws:sso:::instance/ssoins-fedcba9876543210"));
    assert!(!verified(
        &changed,
        "aws_accountaccess_application.sample0",
        "identity_source.identity_center.instance_arn"
    ));
}

#[test]
fn principal_requires_current_application_and_store_in_its_scope() {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/accountaccess-configuration-plan.json"
    ))
    .unwrap();
    let entitlement = "aws_accountaccess_entitlement.sample1";
    let principal = "entitlement.principal_role.principal.identity_center.user_id";
    assert!(verified(&fixture, entitlement, principal));

    let mut unknown = fixture.clone();
    unknown["resource_changes"][3]["change"]["after_unknown"] =
        json!({"identity_store_ids": [true]});
    assert!(!verified(&unknown, entitlement, principal));

    let mut foreign = fixture["configuration"]["provider_config"]["aws"].clone();
    foreign["expressions"]["region"] = json!({"constant_value": "us-west-2"});
    fixture["configuration"]["provider_config"]["aws.foreign"] = foreign;
    for index in [0, 3] {
        fixture["configuration"]["root_module"]["resources"][index]["provider_config_key"] =
            json!("aws.foreign");
    }
    assert!(!verified(&fixture, entitlement, "application_arn"));
    assert!(!verified(&fixture, entitlement, principal));
}
