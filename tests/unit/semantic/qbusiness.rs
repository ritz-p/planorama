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
        "../../fixtures/qbusiness-application-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_qbusiness_application.sample1", "iam_service_role_arn"),
        (
            "aws_qbusiness_application.sample1",
            "identity_center_instance_arn",
        ),
        (
            "aws_qbusiness_application.sample1",
            "encryption_configuration.kms_key_id",
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
    changed["resource_changes"][0]["change"]["after"]["customer_master_key_spec"] =
        json!("RSA_2048");
    assert!(!verified(
        &changed,
        "aws_qbusiness_application.sample1",
        "encryption_configuration.kms_key_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after_unknown"] =
        json!({"customer_master_key_spec":true});
    assert!(!verified(
        &changed,
        "aws_qbusiness_application.sample1",
        "encryption_configuration.kms_key_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["arns"]
        .as_array_mut()
        .unwrap()
        .push(json!("arn:aws:sso:::instance/ssoins-fedcba9876543210"));
    assert!(!verified(
        &changed,
        "aws_qbusiness_application.sample1",
        "identity_center_instance_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][2]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_qbusiness_application.sample1",
        "iam_service_role_arn"
    ));
}

#[test]
fn qbusiness_has_independent_spatial_identity_and_no_computed_sso_child() {
    let fixture = include_str!("../../fixtures/qbusiness-application-plan.json");
    let graph = semantic::transform(&plan::parse(fixture).unwrap());
    let app = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_qbusiness_application")
        .unwrap();
    assert_eq!(app.placement, crate::model::Placement::Spatial);
    assert!(app.member_owner.is_none());
    assert_eq!(graph.coverage.selected.resources, 4);
    assert!(
        !graph
            .nodes
            .iter()
            .any(|n| n.resource_type == "aws_ssoadmin_application")
    );
}
