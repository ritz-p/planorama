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
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/audit-manager-plan.json")).unwrap();
    for (address, path) in [
        ("aws_auditmanager_account_registration.sample0", "kms_key"),
        (
            "aws_auditmanager_account_registration.sample0",
            "delegated_admin_account",
        ),
        ("aws_auditmanager_assessment.sample3", "framework_id"),
        ("aws_auditmanager_assessment.sample3", "roles.role_arn"),
        (
            "aws_auditmanager_assessment.sample3",
            "scope.aws_accounts.id",
        ),
        (
            "aws_auditmanager_assessment.sample3",
            "assessment_reports_destination.destination",
        ),
        (
            "aws_auditmanager_assessment_delegation.sample7",
            "assessment_id",
        ),
        (
            "aws_auditmanager_assessment_report.sample8",
            "assessment_id",
        ),
        ("aws_auditmanager_assessment_delegation.sample7", "role_arn"),
        (
            "aws_auditmanager_framework.sample4",
            "control_sets.controls.id",
        ),
        ("aws_auditmanager_framework_share.sample10", "framework_id"),
        (
            "aws_auditmanager_framework_share.sample10",
            "destination_account",
        ),
        (
            "aws_auditmanager_organization_admin_account_registration.sample11",
            "admin_account_id",
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
        entry["change"]["after_sensitive"] = json!(true);
        assert!(
            !verified(&masked, address, path),
            "{address}/{path} sensitive"
        );
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(false);
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(
        &changed,
        "aws_auditmanager_account_registration.sample0",
        "kms_key"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_auditmanager_account_registration.sample0",
        "kms_key"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["arn"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_auditmanager_account_registration.sample0",
        "kms_key"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_auditmanager_account_registration.sample0",
        "kms_key"
    ));
}
