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
        "../../fixtures/signer-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_signer_signing_profile.sample0",
            "signing_material.certificate_arn",
        ),
        (
            "aws_signer_signing_profile_permission.sample1",
            "profile_name",
        ),
        ("aws_signer_signing_job.sample3", "profile_name"),
        ("aws_signer_signing_job.sample3", "source.s3.bucket"),
        ("aws_signer_signing_job.sample3", "destination.s3.bucket"),
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
    changed["resource_changes"][1]["change"]["after"]["profile_version"] = json!("other-version");
    assert!(!verified(
        &changed,
        "aws_signer_signing_profile_permission.sample1",
        "profile_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_signer_signing_job.sample3",
        "profile_name"
    ));
}

#[test]
fn permissions_without_a_version_link_to_the_named_profile() {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/signer-configuration-plan.json"
    ))
    .unwrap();
    let permission = "aws_signer_signing_profile_permission.sample1";
    fixture["resource_changes"][1]["change"]["after"]
        .as_object_mut()
        .unwrap()
        .remove("profile_version");
    assert!(verified(&fixture, permission, "profile_name"));
    fixture["resource_changes"][1]["change"]["after_unknown"] = json!({"profile_version": true});
    assert!(!verified(&fixture, permission, "profile_name"));
}
