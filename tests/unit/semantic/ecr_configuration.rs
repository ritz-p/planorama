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
        serde_json::from_str(include_str!("../../fixtures/ecr-configuration-plan.json")).unwrap();
    for (address, path) in [
        ("aws_ecrpublic_repository_policy.sample1", "repository_name"),
        (
            "aws_ecr_repository.sample2",
            "encryption_configuration.kms_key",
        ),
        (
            "aws_ecr_repository_creation_template.sample3",
            "encryption_configuration.kms_key",
        ),
        (
            "aws_ecr_repository_creation_template.sample3",
            "custom_role_arn",
        ),
        ("aws_ecr_pull_through_cache_rule.sample4", "credential_arn"),
        ("aws_ecr_pull_through_cache_rule.sample4", "custom_role_arn"),
        (
            "aws_ecr_pull_time_update_exclusion.sample5",
            "principal_arn",
        ),
        (
            "aws_ecr_pull_time_update_exclusion.sample10",
            "principal_arn",
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
    changed["resource_changes"][2]["change"]["after"]["encryption_configuration"][0]["encryption_type"] =
        json!("AES256");
    assert!(!verified(
        &changed,
        "aws_ecr_repository.sample2",
        "encryption_configuration.kms_key"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ecrpublic_repository_policy.sample1",
        "repository_name"
    ));
}
